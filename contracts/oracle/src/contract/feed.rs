use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use finance::{
    currency::{self, AnyVisitorPair, Currency, SymbolOwned},
    price::{dto::BasePrice, Price},
};
use marketprice::{config::Config, market_price::PriceFeeds, SpotPrice};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage, Timestamp},
};
use swap::{SwapGroup, SwapTarget};

use crate::{
    msg::AlarmsStatusResponse,
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
            !tree.query_supported_pairs().any(
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
        tree: &'a SupportedPairs<OracleBase>,
        at: Timestamp,
        total_feeders: usize,
    ) -> Result<impl Iterator<Item = BasePrice<OracleBase, SwapGroup>> + 'a, ContractError> {
        struct LegCmd<'a, 'b, OracleBase>
        where
            OracleBase: Currency,
        {
            feeds: &'a PriceFeeds<'static>,
            storage: &'a dyn Storage,
            at: Timestamp,
            total_feeders: usize,
            stack: &'b mut Vec<BasePrice<OracleBase, SwapGroup>>,
            _base: PhantomData<OracleBase>,
        }

        impl<'a, 'b, OracleBase> AnyVisitorPair for LegCmd<'a, 'b, OracleBase>
        where
            OracleBase: Currency + DeserializeOwned,
        {
            type Output = BasePrice<OracleBase, SwapGroup>;
            type Error = ContractError;
            fn on<B, Q>(self) -> Result<Self::Output, Self::Error>
            where
                B: Currency + DeserializeOwned,
                Q: Currency + DeserializeOwned,
            {
                let price: BasePrice<OracleBase, SwapGroup> = loop {
                    match self
                        .stack
                        .last()
                        .map(TryInto::<Price<Q, OracleBase>>::try_into)
                    {
                        None => {
                            break self.feeds.price_of_feed::<B, OracleBase>(
                                self.storage,
                                self.at,
                                self.total_feeders,
                            )?
                        }
                        Some(Ok(price_parent)) => {
                            break self.feeds.price_of_feed::<B, Q>(
                                self.storage,
                                self.at,
                                self.total_feeders,
                            )? * price_parent
                        }
                        _ => {
                            self.stack.pop();
                        }
                    }
                }
                .into();
                self.stack.push(price.clone());

                Ok(price)
            }
        }

        let res = tree.query_supported_pairs().scan(
            vec![],
            move |stack: &mut Vec<BasePrice<OracleBase, SwapGroup>>, leg: SwapLeg| {
                let res = currency::visit_any_on_tickers::<SwapGroup, SwapGroup, _>(
                    &leg.from,
                    &leg.to.target,
                    LegCmd {
                        feeds: &self.feeds,
                        storage,
                        at,
                        total_feeders,
                        stack,
                        _base: PhantomData::<OracleBase>,
                    },
                )
                .expect("price calculation error");
                Some(res)
            },
        );
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
    let batch = Batch::default();
    let tree = SupportedPairs::load(storage)?;
    let prices = calc_all_prices::<OracleBase>(storage, block_time, &tree)?;
    let mut alarms = MarketAlarms::load(storage)?;
    let response = alarms.try_notify_alarms::<OracleBase>(storage, batch, prices, max_count)?;
    alarms.save(storage)?;
    Ok(response)
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
) -> Result<impl Iterator<Item = BasePrice<OracleBase, SwapGroup>> + 'a, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let total_registered = Feeders::total_registered(storage)?;
    use crate::state::config::Config as OracleConfig;
    let config = OracleConfig::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config.price_config);
    oracle.all_prices_iter(storage, tree, block_time, total_registered)
}

#[cfg(test)]
mod test {
    use super::*;
    use ::currency::{
        lease::{Atom, Cro, Juno, Osmo, Wbtc, Weth},
        lpn::Usdc,
    };
    use finance::{coin::Coin, duration::Duration, percent::Percent, price};
    use sdk::cosmwasm_std::testing::{self, MockStorage};
    use tree::HumanReadableTree;

    type TheCurrency = Usdc;

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

    #[test]
    fn all_prices_iter() {
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
                    price::total_of(Coin::<Wbtc>::new(1))
                        .is(Coin::<TheCurrency>::new(1))
                        .into(),
                    price::total_of(Coin::<Atom>::new(2))
                        .is(Coin::<TheCurrency>::new(1))
                        .into(),
                    price::total_of(Coin::<Weth>::new(1))
                        .is(Coin::<Wbtc>::new(1))
                        .into(),
                    price::total_of(Coin::<Osmo>::new(1))
                        .is(Coin::<Atom>::new(1))
                        .into(),
                    price::total_of(Coin::<Cro>::new(3))
                        .is(Coin::<Osmo>::new(1))
                        .into(),
                    price::total_of(Coin::<Juno>::new(1))
                        .is(Coin::<Osmo>::new(1))
                        .into(),
                ],
            )
            .unwrap();

        let prices: Vec<_> = oracle
            .all_prices_iter(&storage, &tree, env.block.time, 1)
            .unwrap()
            .collect();

        let expected: Vec<BasePrice<TheCurrency, SwapGroup>> = vec![
            price::total_of(Coin::<Wbtc>::new(1))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
            price::total_of(Coin::<Weth>::new(1))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
            price::total_of(Coin::<Atom>::new(2))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
            price::total_of(Coin::<Osmo>::new(2))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
            price::total_of(Coin::<Juno>::new(2))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
            price::total_of(Coin::<Cro>::new(6))
                .is(Coin::<TheCurrency>::new(1))
                .into(),
        ];

        assert_eq!(expected, prices);
    }
}
