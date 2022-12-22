use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::currency::{Currency, SymbolOwned};
use marketprice::{
    market_price::{Config as PriceConfig, PriceFeeds},
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
    state::{
        supported_pairs::{SupportedPairs, SwapLeg},
        Config,
    },
    ContractError,
};

use super::{alarms::MarketAlarms, feeder::Feeders};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Feeds<OracleBase> {
    config: Config,
    _base: PhantomData<OracleBase>,
}

impl<OracleBase> Feeds<OracleBase>
where
    OracleBase: Currency + DeserializeOwned,
{
    const MARKET_PRICE: PriceFeeds<'static> = PriceFeeds::new("market_price");

    pub fn with(config: Config) -> Self {
        Self {
            config,
            _base: PhantomData,
        }
    }

    pub fn get_prices(
        &self,
        storage: &dyn Storage,
        config: &PriceConfig,
        at: Timestamp,
        currencies: &[SymbolOwned],
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for currency in currencies {
            let path = tree.load_path(currency)?;
            let leaf_to_root = path.iter().map(|owned| owned.as_str());
            let price =
                Self::MARKET_PRICE.price::<OracleBase, _>(storage, config, at, leaf_to_root)?;
            prices.push(price);
        }
        Ok(prices)
    }

    fn feed_prices(
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

        Self::MARKET_PRICE.feed(
            storage,
            block_time,
            sender_raw,
            prices,
            self.config.price_feed_period,
        )?;

        Ok(())
    }
}

pub fn try_feed_prices<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<SpotPrice>,
) -> Result<Response, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config);

    if !prices.is_empty() {
        // Store the new price feed
        oracle.feed_prices(storage, block_time, &sender_raw, &prices)?;
    }

    Ok(Response::default())
}

// TODO: optimize
pub fn get_all_prices<OracleBase>(
    storage: &dyn Storage,
    at: Timestamp,
) -> Result<Vec<SpotPrice>, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;

    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config);
    let price_config = Feeders::price_config(storage, &oracle.config)?;

    let mut prices = vec![];
    for leg in SupportedPairs::<OracleBase>::load(storage)?
        .query_supported_pairs()
        .into_iter()
    {
        let path = tree.load_path(&leg.from)?;
        let path = path.iter().map(|owned| owned.as_str());
        // we need to gather all available prices without NoPrice error
        if let Ok(price) = Feeds::<OracleBase>::MARKET_PRICE.price::<OracleBase, _>(
            storage,
            &price_config,
            at,
            path,
        ) {
            prices.push(price);
        }
    }
    Ok(prices)
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
    let prices = get_all_prices::<OracleBase>(storage, block_time)?;
    MarketAlarms::try_notify_alarms(storage, batch, &prices, max_count)
}

pub fn try_query_alarms<OracleBase>(
    storage: &dyn Storage,
    block_time: Timestamp,
) -> Result<AlarmsStatusResponse, ContractError>
where
    OracleBase: Currency + DeserializeOwned,
{
    let prices = get_all_prices::<OracleBase>(storage, block_time)?;
    MarketAlarms::try_query_alarms(storage, &prices)
}
