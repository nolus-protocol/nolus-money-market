use std::marker::PhantomData;

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf, PairsGroup};
use finance::price::{base::BasePrice, dto::PriceDTO};
use marketprice::{config::Config, market_price::PriceFeeds};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    api::{swap::SwapTarget, SwapLeg},
    error::ContractError,
    state::supported_pairs::SupportedPairs,
};

use self::{leg_cmd::LegCmd, price_querier::FedPrices};

use super::PriceResult;

mod leg_cmd;
mod price_querier;

pub struct Feeds<PriceG, BaseC, BaseG> {
    feeds: PriceFeeds<'static, PriceG>,
    _base_c: PhantomData<BaseC>,
    _base_g: PhantomData<BaseG>,
}

impl<PriceG, BaseC, BaseG> Feeds<PriceG, BaseC, BaseG>
where
    PriceG: Group + PairsGroup<CommonGroup = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
{
    pub(crate) fn with(config: Config) -> Self {
        Self {
            feeds: PriceFeeds::new("market_price", config),
            _base_c: PhantomData,
            _base_g: PhantomData,
        }
    }

    pub(crate) fn feed_prices(
        &self,
        storage: &mut dyn Storage,
        block_time: Timestamp,
        sender_raw: &Addr,
        prices: &[PriceDTO<PriceG, PriceG>],
    ) -> Result<(), ContractError> {
        let tree = SupportedPairs::<PriceG, BaseC>::load(storage)?;
        if prices.iter().any(|price| {
            !tree.swap_pairs_df().any(
                |SwapLeg {
                     from,
                     to: SwapTarget { target: to, .. },
                 }| {
                    price.base().currency() == from && price.quote().currency() == to
                },
            )
        }) {
            return Err(ContractError::UnsupportedDenomPairs {});
        }

        self.feeds.feed(storage, block_time, sender_raw, prices)?;

        Ok(())
    }

    pub fn all_prices_iter<'r, 'self_, 'storage, I>(
        &'self_ self,
        storage: &'storage dyn Storage,
        swap_pairs_df: I,
        at: Timestamp,
        total_feeders: usize,
    ) -> impl Iterator<Item = PriceResult<PriceG, BaseC, BaseG>> + 'r
    where
        'self_: 'r,
        'storage: 'r,
        I: Iterator<Item = SwapLeg<PriceG>> + 'r,
    {
        let cmd: LegCmd<PriceG, BaseC, BaseG, FedPrices<'_, PriceG>> =
            LegCmd::new(FedPrices::new(storage, &self.feeds, at, total_feeders));

        swap_pairs_df
            .scan(cmd, |cmd, leg: SwapLeg<PriceG>| {
                Some(currency::visit_any_on_currencies(leg.from, leg.to.target, cmd).transpose())
            })
            .flatten()
    }

    pub fn calc_base_price(
        &self,
        storage: &dyn Storage,
        tree: &SupportedPairs<PriceG, BaseC>,
        currency: &CurrencyDTO<PriceG>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<BasePrice<PriceG, BaseC, BaseG>, ContractError> {
        let dto = self
            .feeds
            .price::<BaseC, _, _>(
                currency::dto::<BaseC, _>(),
                storage,
                at,
                total_feeders,
                tree.load_path(currency)?,
            )
            .map_err(Into::<ContractError>::into)?;
        BasePrice::from_dto_price(dto, &currency::dto::<BaseC, _>()).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use currencies::{
        Lpn as BaseCurrency, PaymentC1, PaymentC3, PaymentC4, PaymentC5, PaymentC6, PaymentC7,
        PaymentGroup as PriceCurrencies,
    };
    use currency::{Currency, SymbolStatic};
    use finance::{
        coin::Amount,
        duration::Duration,
        percent::Percent,
        price::{dto::PriceDTO, Price},
    };
    use marketprice::alarms::prefix::Prefix;
    use price_querier::PriceQuerier;
    use sdk::cosmwasm_std::{
        self,
        testing::{self, MockStorage},
    };
    use tree::HumanReadableTree;

    use crate::tests;

    use super::*;

    #[derive(Clone)]
    pub struct TestFeeds(
        pub HashMap<(SymbolStatic, SymbolStatic), PriceDTO<PriceCurrencies, PriceCurrencies>>,
    );
    impl TestFeeds {
        pub fn add<B, Q>(&mut self, total_of: Amount, is: Amount)
        where
            B: CurrencyDef,
            B::Group: MemberOf<PriceCurrencies>,
            Q: CurrencyDef,
            Q::Group: MemberOf<PriceCurrencies>,
        {
            self.0.insert(
                (B::ticker(), Q::ticker()),
                tests::dto_price::<B, PriceCurrencies, Q, PriceCurrencies>(total_of, is),
            );
        }
    }

    impl PriceQuerier for TestFeeds {
        type CurrencyGroup = PriceCurrencies;

        fn price<C, QuoteC>(
            &self,
            amount_c: &CurrencyDTO<Self::CurrencyGroup>,
            quote_c: &CurrencyDTO<Self::CurrencyGroup>,
        ) -> Result<Option<Price<C, QuoteC>>, ContractError>
        where
            C: Currency + MemberOf<Self::CurrencyGroup>,
            QuoteC: Currency + MemberOf<Self::CurrencyGroup>,
        {
            Ok(self
                .0
                .get(&(amount_c.first_key(), quote_c.first_key()))
                .map(|dto| dto.as_specific(amount_c, quote_c)))
        }
    }

    fn test_case() -> HumanReadableTree<SwapTarget<PriceCurrencies>> {
        let base = BaseCurrency::ticker();
        let osmo = PaymentC5::ticker();
        let nls = PaymentC1::ticker();
        let weth = PaymentC7::ticker();
        let atom = PaymentC3::ticker();
        let axl = PaymentC4::ticker();
        let cro = PaymentC6::ticker();

        cosmwasm_std::from_json(format!(
            r#"
            {{
                "value":[0,"{base}"],
                "children":[
                    {{
                        "value":[4,"{atom}"],
                        "children":[
                            {{"value":[3,"{weth}"]}}
                        ]
                    }},
                    {{
                        "value":[2,"{nls}"],
                        "children":[
                            {{
                                "value":[1,"{osmo}"],
                                "children":[
                                    {{"value":[5,"{axl}"]}},
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
        use currencies::{Lpns as BaseCurrencies, PaymentGroup as PriceCurrencies};
        use finance::price::base::BasePrice;

        use super::*;

        #[test]
        fn normal() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_case();
            let tree = SupportedPairs::<PriceCurrencies, BaseCurrency>::new::<BaseCurrency>(
                tree.into_tree(),
            )
            .unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<PriceCurrencies, BaseCurrency, BaseCurrencies> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    env.block.time,
                    &Addr::unchecked("feeder"),
                    &[
                        tests::dto_price::<PaymentC3, _, BaseCurrency, _>(1, 1),
                        tests::dto_price::<PaymentC1, _, BaseCurrency, _>(2, 1),
                        tests::dto_price::<PaymentC7, _, PaymentC3, _>(1, 1),
                        tests::dto_price::<PaymentC5, _, PaymentC1, _>(1, 1),
                        tests::dto_price::<PaymentC6, _, PaymentC5, _>(3, 1),
                        tests::dto_price::<PaymentC4, _, PaymentC5, _>(1, 1),
                    ],
                )
                .unwrap();

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .flatten()
                .collect();

            let expected: Vec<BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>> = vec![
                tests::base_price::<PaymentC3>(1, 1),
                tests::base_price::<PaymentC7>(1, 1),
                tests::base_price::<PaymentC1>(2, 1),
                tests::base_price::<PaymentC5>(2, 1),
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC6>(6, 1),
            ];

            assert_eq!(expected, prices);
        }

        #[test]
        fn missing_price() {
            let mut storage = MockStorage::new();
            let env = testing::mock_env();
            let tree = test_case();
            let tree = SupportedPairs::<PriceCurrencies, BaseCurrency>::new::<BaseCurrency>(
                tree.into_tree(),
            )
            .unwrap();
            tree.save(&mut storage).unwrap();

            let config = Config::new(
                Percent::HUNDRED,
                Duration::from_secs(5),
                10,
                Percent::from_percent(50),
            );

            let oracle: Feeds<PriceCurrencies, BaseCurrency, BaseCurrencies> = Feeds::with(config);

            oracle
                .feed_prices(
                    &mut storage,
                    env.block.time,
                    &Addr::unchecked("feeder"),
                    &[
                        // tests::dto_price::<PaymentC3, BaseCurrency>(1, 1),
                        tests::dto_price::<PaymentC1, _, BaseCurrency, _>(2, 1),
                        tests::dto_price::<PaymentC7, _, PaymentC3, _>(1, 1),
                        tests::dto_price::<PaymentC5, _, PaymentC1, _>(1, 1),
                        tests::dto_price::<PaymentC6, _, PaymentC5, _>(3, 1),
                        tests::dto_price::<PaymentC4, _, PaymentC5, _>(1, 1),
                    ],
                )
                .unwrap();

            let expected: Vec<BasePrice<PriceCurrencies, BaseCurrency, BaseCurrencies>> = vec![
                tests::base_price::<PaymentC1>(2, 1),
                tests::base_price::<PaymentC5>(2, 1),
                tests::base_price::<PaymentC4>(2, 1),
                tests::base_price::<PaymentC6>(6, 1),
            ];

            let prices: Vec<_> = oracle
                .all_prices_iter(&storage, tree.swap_pairs_df(), env.block.time, 1)
                .collect::<Result<_, _>>()
                .unwrap();

            assert_eq!(prices, expected);
        }
    }
}
