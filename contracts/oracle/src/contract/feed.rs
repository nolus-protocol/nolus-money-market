use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use finance::currency::{Currency, SymbolOwned};
use marketprice::{
    market_price::{Config, PriceFeeds},
    SpotPrice,
};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage, Timestamp},
};
use swap::SwapTarget;

use crate::{
    msg::AlarmsStatusResponse,
    state::supported_pairs::{SupportedPairs, SwapLeg},
    ContractError,
};

use super::{alarms::MarketAlarms, feeder::Feeders};

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
        let supported_pairs = SupportedPairs::<OracleBase>::load(storage)?.query_supported_pairs();
        if prices.iter().any(|price| {
            !supported_pairs.iter().any(
                |SwapLeg {
                     from,
                     to: SwapTarget { target: to, .. },
                 }| {
                    price.base().ticker() == from && price.quote().ticker() == to
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
        currencies: &[SymbolOwned],
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for currency in currencies {
            let price = self.calc_price(&tree, storage, currency, at)?;
            prices.push(price);
        }
        Ok(prices)
    }

    fn calc_all_prices(
        &self,
        storage: &dyn Storage,
        at: Timestamp,
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for leg in tree.clone().query_supported_pairs() {
            if let Ok(price) = self.calc_price(&tree, storage, &leg.from, at) {
                prices.push(price);
            }
        }
        Ok(prices)
    }

    fn calc_price(
        &self,
        tree: &SupportedPairs<OracleBase>,
        storage: &dyn Storage,
        currency: &SymbolOwned,
        at: Timestamp,
    ) -> Result<SpotPrice, ContractError> {
        let path = tree.load_path(currency)?;
        let leaf_to_root = path.iter().map(|owned| owned.as_str());
        let price = self
            .feeds
            .price::<OracleBase, _>(storage, at, leaf_to_root)?;
        Ok(price)
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
    let prices = calc_all_prices::<OracleBase>(storage, block_time)?;
    MarketAlarms::try_notify_alarms(storage, batch, &prices, max_count)
}

pub fn try_query_alarms<OracleBase>(
    storage: &dyn Storage,
    block_time: Timestamp,
) -> Result<AlarmsStatusResponse, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let prices = calc_all_prices::<OracleBase>(storage, block_time)?;
    MarketAlarms::try_query_alarms(storage, &prices)
}

fn calc_all_prices<OracleBase>(
    storage: &dyn Storage,
    block_time: Timestamp,
) -> Result<Vec<SpotPrice>, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    use crate::state::Config;
    let config = Config::load(storage)?;
    let price_config = Feeders::price_config(storage, &config)?;
    let oracle = Feeds::<OracleBase>::with(price_config);
    oracle.calc_all_prices(storage, block_time)
}
