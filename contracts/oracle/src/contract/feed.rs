use std::marker::PhantomData;

use ::currency::native::Nls;
use platform::batch::Batch;
use serde::de::DeserializeOwned;

use finance::{
    currency::{self, AnyVisitorPair, Currency, SymbolOwned},
    price::{base::BasePrice, Price},
};
use marketprice::{config::Config, error::PriceFeedsError, market_price::PriceFeeds, SpotPrice};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage, Timestamp},
};
use swap::{SwapGroup, SwapTarget};

use crate::{
    msg::{AlarmsStatusResponse, ExecuteAlarmMsg},
    state::supported_pairs::{SupportedPairs, SwapLeg},
    ContractError,
};

use super::{alarms::MarketAlarms, feeder::Feeders};

pub struct Feeds<OracleBase>
where
    OracleBase: Currency + DeserializeOwned,
{
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

    pub(crate) fn calc_prices(
        &self,
        storage: &dyn Storage,
        at: Timestamp,
        total_feeders: usize,
        currencies: &[SymbolOwned],
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for currency in currencies {
            let price = self.calc_price(&tree, storage, currency, at, total_feeders)?;
            prices.push(price);
        }
        Ok(prices)
    }

    pub fn all_prices_iter<'a>(
        self,
        storage: &'a dyn Storage,
        swap_pairs_df: impl Iterator<Item = SwapLeg> + 'a,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<impl Iterator<Item = BasePrice<SwapGroup, OracleBase>> + 'a, ContractError> {
        let cmd = LegCmd {
            price_querier: ConfiguredFeeds {
                feeds: self.feeds,
                storage,
                at,
                total_feeders,
            },
            stack: vec![],
            err: false,
        };
        let res = swap_pairs_df
            .scan(cmd, |cmd, leg| {
                let res = currency::visit_any_on_tickers::<SwapGroup, SwapGroup, _>(
                    &leg.from,
                    &leg.to.target,
                    cmd,
                )
                .expect("price calculation error");
                Some(res)
            })
            // TODO: process errors
            .flatten();
        Ok(res)
    }

    fn calc_price(
        &self,
        tree: &SupportedPairs<OracleBase>,
        storage: &dyn Storage,
        currency: &SymbolOwned,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<SpotPrice, ContractError> {
        self.feeds
            .price::<OracleBase, _>(storage, at, total_feeders, tree.load_path(currency)?)
            .map_err(Into::into)
    }
}

pub fn try_notify_alarms<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    max_count: u32,
) -> Result<Response, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let tree = SupportedPairs::load(storage)?;
    let prices = calc_all_prices::<OracleBase>(storage, block_time, &tree)?;
    let batch =
        MarketAlarms::notify_alarms_iter::<OracleBase>(storage, prices, max_count.try_into()?)
            .try_fold(
                Batch::default(),
                |mut batch, receiver| -> Result<Batch, ContractError> {
                    // TODO: get rid of the Nls dummy type argument
                    batch.schedule_execute_wasm_reply_always::<_, Nls>(
                        &receiver?,
                        ExecuteAlarmMsg::PriceAlarm(),
                        None,
                        batch.len().try_into()?,
                    )?;
                    Ok(batch)
                },
            )?;

    Ok(batch.into())
}

pub fn try_query_alarms<OracleBase>(
    storage: &dyn Storage,
    block_time: Timestamp,
) -> Result<AlarmsStatusResponse, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let tree = SupportedPairs::load(storage)?;
    let prices = calc_all_prices::<OracleBase>(storage, block_time, &tree)?;
    MarketAlarms::try_query_alarms::<OracleBase>(storage, prices)
}

fn calc_all_prices<'a, OracleBase>(
    storage: &'a dyn Storage,
    block_time: Timestamp,
    tree: &'a SupportedPairs<OracleBase>,
) -> Result<impl Iterator<Item = BasePrice<SwapGroup, OracleBase>> + 'a, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let total_registered = Feeders::total_registered(storage)?;
    use crate::state::config::Config as OracleConfig;
    let config = OracleConfig::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config.price_config);
    oracle.all_prices_iter(storage, tree.swap_pairs_df(), block_time, total_registered)
}

struct LegCmd<OracleBase, Querier>
where
    OracleBase: Currency,
    Querier: PriceQuerier,
{
    price_querier: Querier,
    stack: Vec<BasePrice<SwapGroup, OracleBase>>,
    err: bool,
}

impl<Querier, OracleBase> LegCmd<OracleBase, Querier>
where
    OracleBase: Currency,
    Querier: PriceQuerier,
{
    // TODO: improve implementation
    fn recover<B, Q>(&mut self) -> bool
    where
        B: Currency,
        Q: Currency,
    {
        if Q::TICKER == OracleBase::TICKER {
            self.stack.clear();
            self.err = false;
        } else {
            let parent_idx = self
                .stack
                .iter()
                .position(|parent| parent.base_ticker() == Q::TICKER);

            if let Some(n) = parent_idx {
                self.stack.truncate(n + 1);
                self.err = false;
            } else {
                return false;
            }
        }
        true
    }
}

impl<OracleBase, Querier> AnyVisitorPair for &mut LegCmd<OracleBase, Querier>
where
    OracleBase: Currency + DeserializeOwned,
    Querier: PriceQuerier,
{
    type Output = Option<BasePrice<SwapGroup, OracleBase>>;
    type Error = ContractError;

    fn on<B, Q>(self) -> Result<Self::Output, Self::Error>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned,
    {
        // recovery mode
        if self.err && !self.recover::<B, Q>() {
            return Ok(None);
        }

        let price: Option<BasePrice<SwapGroup, OracleBase>> = loop {
            match self
                .stack
                .last()
                .map(TryInto::<Price<Q, OracleBase>>::try_into)
            {
                None => {
                    debug_assert_eq!(Q::TICKER, OracleBase::TICKER);

                    break self.price_querier.price::<B, OracleBase>()?;
                }
                Some(Ok(price_parent)) => {
                    break self
                        .price_querier
                        .price::<B, Q>()?
                        .map(|price| price * price_parent)
                }
                _ => {
                    self.stack.truncate(self.stack.len() - 1);
                }
            }
        }
        .map(|price| {
            let bprice: BasePrice<SwapGroup, OracleBase> = price.into();
            self.stack.push(bprice.clone());
            bprice
        });

        if price.is_none() {
            self.err = true;
        }

        Ok(price)
    }
}

// TODO: rename to something meaningfull
struct ConfiguredFeeds<'a> {
    feeds: PriceFeeds<'static>,
    at: Timestamp,
    total_feeders: usize,
    storage: &'a dyn Storage,
}

trait PriceQuerier {
    fn price<B, Q>(&self) -> Result<Option<Price<B, Q>>, ContractError>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned;
}

impl<'a> PriceQuerier for ConfiguredFeeds<'a> {
    fn price<B, Q>(&self) -> Result<Option<Price<B, Q>>, ContractError>
    where
        B: Currency + DeserializeOwned,
        Q: Currency + DeserializeOwned,
    {
        let price = self
            .feeds
            .price_of_feed(self.storage, self.at, self.total_feeders)
            .map(Some)
            .or_else(|err| match err {
                PriceFeedsError::NoPrice() => Ok(None),
                _ => Err(err),
            })?;
        Ok(price)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::tests::{self, TheCurrency};
    use ::currency::lease::{Atom, Cro, Juno, Osmo, Wbtc, Weth};
    use finance::{
        currency::SymbolStatic, duration::Duration, percent::Percent, price::dto::PriceDTO,
    };
    use sdk::cosmwasm_std::testing::{self, MockStorage};
    use tree::HumanReadableTree;

    #[derive(Clone)]
    struct TestFeeds(HashMap<(SymbolStatic, SymbolStatic), PriceDTO<SwapGroup, SwapGroup>>);
    impl TestFeeds {
        fn add<B, Q>(&mut self, total_of: u128, is: u128)
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
                .unwrap()
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
                .unwrap()
                .collect();

            assert_eq!(expected, prices);
        }

        #[test]
        fn leg_cmd_normal() {
            let mut feeds = TestFeeds(HashMap::new());
            feeds.add::<Wbtc, TheCurrency>(1, 1);
            feeds.add::<Atom, TheCurrency>(2, 1);
            feeds.add::<Weth, Wbtc>(2, 1);

            let mut cmd = LegCmd::<TheCurrency, _> {
                price_querier: feeds.clone(),
                stack: vec![],
                err: false,
            };
            assert_eq!(
                cmd.on::<Wbtc, TheCurrency>(),
                Ok(Some(tests::base_price::<Wbtc>(1, 1)))
            );
            assert_eq!(cmd.stack, vec![tests::base_price::<Wbtc>(1, 1)]);
            assert!(!cmd.err);

            // child
            assert_eq!(
                cmd.on::<Weth, Wbtc>(),
                Ok(Some(tests::base_price::<Weth>(2, 1)))
            );
            assert_eq!(
                cmd.stack,
                vec![
                    tests::base_price::<Wbtc>(1, 1),
                    tests::base_price::<Weth>(2, 1)
                ]
            );
            assert!(!cmd.err);

            // hop to the next branch
            assert_eq!(
                cmd.on::<Atom, TheCurrency>(),
                Ok(Some(tests::base_price::<Atom>(2, 1)))
            );
            assert_eq!(cmd.stack, vec![tests::base_price::<Atom>(2, 1)]);
            assert!(!cmd.err);
        }

        #[test]
        fn leg_cmd_missing_price() {
            let mut feeds = TestFeeds(HashMap::new());
            feeds.add::<Wbtc, TheCurrency>(1, 1);
            feeds.add::<Atom, TheCurrency>(2, 1);
            feeds.add::<Weth, Wbtc>(2, 1);
            feeds.add::<Osmo, Weth>(1, 1);
            feeds.add::<Cro, Osmo>(3, 1);

            feeds.add::<Juno, Wbtc>(1, 1);

            let mut cmd = LegCmd::<TheCurrency, _> {
                price_querier: feeds.clone(),
                stack: vec![
                    tests::base_price::<Wbtc>(1, 1),
                    tests::base_price::<Weth>(2, 1),
                ],
                err: false,
            };

            // no price
            assert_eq!(cmd.on::<Cro, Weth>(), Ok(None));
            assert_eq!(
                cmd.stack,
                vec![
                    tests::base_price::<Wbtc>(1, 1),
                    tests::base_price::<Weth>(2, 1)
                ]
            );
            assert!(cmd.err);

            // recover, hop to the top child, clean the stack
            assert_eq!(
                cmd.on::<Atom, TheCurrency>(),
                Ok(Some(tests::base_price::<Atom>(2, 1)))
            );
            assert_eq!(cmd.stack, vec![tests::base_price::<Atom>(2, 1)]);
            assert!(!cmd.err);

            let mut cmd = LegCmd::<TheCurrency, _> {
                price_querier: feeds.clone(),
                stack: vec![
                    tests::base_price::<Wbtc>(1, 1),
                    tests::base_price::<Weth>(2, 1),
                ],
                err: true,
            };

            // recover, hop to the close child, clean the stack
            assert_eq!(
                cmd.on::<Juno, Wbtc>(),
                Ok(Some(tests::base_price::<Juno>(1, 1)))
            );
            assert_eq!(
                cmd.stack,
                vec![
                    tests::base_price::<Wbtc>(1, 1),
                    tests::base_price::<Juno>(1, 1)
                ]
            );
            assert!(!cmd.err);
        }
    }
}
