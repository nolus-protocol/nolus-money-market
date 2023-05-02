use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use finance::currency::{self, Currency, SymbolOwned};
use leg_cmd::LegCmd;
use marketprice::{config::Config, market_price::PriceFeeds, SpotPrice};
use price_querier::FedPrices;
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};
use swap::{SwapGroup, SwapTarget};

use crate::{
    contract::alarms::PriceResult,
    state::supported_pairs::{SupportedPairs, SwapLeg},
    ContractError,
};

mod leg_cmd;
mod price_querier;

pub struct Feeds<OracleBase> {
    feeds: PriceFeeds<'static>,
    _base: PhantomData<OracleBase>,
}

impl<OracleBase> Feeds<OracleBase>
where
    OracleBase: Currency + DeserializeOwned,
{
    pub(crate) fn with(config: Config) -> Self {
        Self {
            feeds: PriceFeeds::new("market_price", config),
            _base: PhantomData,
        }
    }

    pub(crate) fn feed_prices(
        &self,
        storage: &mut dyn Storage,
        block_time: Timestamp,
        sender_raw: &Addr,
        prices: &[SpotPrice],
    ) -> Result<(), ContractError> {
        let tree = SupportedPairs::<OracleBase>::load(storage)?;
        if prices.iter().any(|price| {
            !tree.swap_pairs_df().any(
                |SwapLeg {
                     from,
                     to: SwapTarget { target: to, .. },
                 }| {
                    price.base().ticker() == &from && price.quote().ticker() == &to
                },
            )
        }) {
            return Err(ContractError::UnsupportedDenomPairs {});
        }

        self.feeds.feed(storage, block_time, sender_raw, prices)?;

        Ok(())
    }

    pub fn all_prices_iter<'a>(
        &'a self,
        storage: &'a dyn Storage,
        swap_pairs_df: impl Iterator<Item = SwapLeg> + 'a,
        at: Timestamp,
        total_feeders: usize,
    ) -> impl Iterator<Item = PriceResult<OracleBase>> + 'a {
        let cmd = LegCmd::new(
            FedPrices::new(storage, &self.feeds, at, total_feeders),
            vec![],
        );
        swap_pairs_df
            .scan(cmd, |cmd, leg| {
                let res = currency::visit_any_on_tickers::<SwapGroup, SwapGroup, _>(
                    &leg.from,
                    &leg.to.target,
                    cmd,
                )
                .transpose();
                Some(res)
            })
            .flatten()
    }

    pub fn calc_price(
        &self,
        storage: &dyn Storage,
        tree: &SupportedPairs<OracleBase>,
        currency: &SymbolOwned,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<SpotPrice, ContractError> {
        self.feeds
            .price::<OracleBase, _>(storage, at, total_feeders, tree.load_path(currency)?)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use ::currency::lease::{Atom, Cro, Juno, Osmo, Wbtc, Weth};
    use finance::{
        coin::Amount,
        currency::SymbolStatic,
        duration::Duration,
        percent::Percent,
        price::{dto::PriceDTO, Price},
    };
    use price_querier::PriceQuerier;
    use sdk::cosmwasm_std::testing::{self, MockStorage};
    use tree::HumanReadableTree;

    use crate::tests::{self, TheCurrency};

    use super::*;

    #[derive(Clone)]
    pub struct TestFeeds(pub HashMap<(SymbolStatic, SymbolStatic), PriceDTO<SwapGroup, SwapGroup>>);
    impl TestFeeds {
        pub fn add<B, Q>(&mut self, total_of: Amount, is: Amount)
        where
            B: Currency,
            Q: Currency,
        {
            self.0.insert(
                (B::TICKER, Q::TICKER),
                tests::dto_price::<B, Q>(total_of, is),
            );
        }
    }

    impl PriceQuerier for TestFeeds {
        fn price<B, Q>(&self) -> Result<Option<Price<B, Q>>, ContractError>
        where
            B: Currency + DeserializeOwned,
            Q: Currency + DeserializeOwned,
        {
            Ok(self
                .0
                .get(&(B::TICKER, Q::TICKER))
                .map(Price::try_from)
                .transpose()?)
        }
    }

    fn test_case() -> HumanReadableTree<SwapTarget> {
        let base = TheCurrency::TICKER;
        let osmo = Osmo::TICKER;
        let atom = Atom::TICKER;
        let weth = Weth::TICKER;
        let wbtc = Wbtc::TICKER;
        let juno = Juno::TICKER;
        let cro = Cro::TICKER;

        serde_json_wasm::from_str(&format!(
            r#"
            {{
                "value":[0,"{base}"],
                "children":[
                    {{
                        "value":[4,"{wbtc}"],
                        "children":[
                            {{"value":[3,"{weth}"]}}
                        ]
                    }},
                    {{
                        "value":[2,"{atom}"],
                        "children":[
                            {{
                                "value":[1,"{osmo}"],
                                "children":[
                                    {{"value":[5,"{juno}"]}},
                                    {{"value":[6,"{cro}"]}}
                                ]
                            }}
                        ]
                    }}
                ]
            }}"#
        ))
        .unwrap()
    }

    mod all_prices_iter {
        use finance::price::base::BasePrice;

        use super::*;

        #[test]
        fn normal() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_case();
            let tree = SupportedPairs::<TheCurrency>::new(tree.into_tree()).unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<TheCurrency> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    env.block.time,
                    &Addr::unchecked("feeder"),
                    &[
                        tests::dto_price::<Wbtc, TheCurrency>(1, 1),
                        tests::dto_price::<Atom, TheCurrency>(2, 1),
                        tests::dto_price::<Weth, Wbtc>(1, 1),
                        tests::dto_price::<Osmo, Atom>(1, 1),
                        tests::dto_price::<Cro, Osmo>(3, 1),
                        tests::dto_price::<Juno, Osmo>(1, 1),
                    ],
                )
                .unwrap();

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .flatten()
                .collect();

            let expected: Vec<BasePrice<SwapGroup, TheCurrency>> = vec![
                tests::base_price::<Wbtc>(1, 1),
                tests::base_price::<Weth>(1, 1),
                tests::base_price::<Atom>(2, 1),
                tests::base_price::<Osmo>(2, 1),
                tests::base_price::<Juno>(2, 1),
                tests::base_price::<Cro>(6, 1),
            ];

            assert_eq!(expected, prices);
        }

        #[test]
        fn missing_price() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_case();
            let tree = SupportedPairs::<TheCurrency>::new(tree.into_tree()).unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<TheCurrency> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    env.block.time,
                    &Addr::unchecked("feeder"),
                    &[
                        // tests::dto_price::<Wbtc, TheCurrency>(1, 1),
                        tests::dto_price::<Atom, TheCurrency>(2, 1),
                        tests::dto_price::<Weth, Wbtc>(1, 1),
                        tests::dto_price::<Osmo, Atom>(1, 1),
                        tests::dto_price::<Cro, Osmo>(3, 1),
                        tests::dto_price::<Juno, Osmo>(1, 1),
                    ],
                )
                .unwrap();

            let expected: Vec<BasePrice<SwapGroup, TheCurrency>> = vec![
                tests::base_price::<Atom>(2, 1),
                tests::base_price::<Osmo>(2, 1),
                tests::base_price::<Juno>(2, 1),
                tests::base_price::<Cro>(6, 1),
            ];

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .flatten()
                .collect();

            assert_eq!(expected, prices);
        }
    }
}
