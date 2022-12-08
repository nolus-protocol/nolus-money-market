use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::native::Nls;
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
    state::{
        supported_pairs::{SupportedPairs, SwapLeg},
        Config,
    },
    ContractError,
};

use super::{feeder::Feeders, alarms::MarketAlarms};

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
        currencies: &[SymbolOwned],
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for currency in currencies {
            let path = tree.load_path(currency)?;
            let leaf_to_root = path.iter().map(|owned| owned.as_str());
            let price = Self::MARKET_PRICE.price::<OracleBase, _>(storage, config, leaf_to_root)?;
            prices.push(price);
        }
        Ok(prices)
    }

    // TODO: optimize
    pub fn get_all_prices(
        &self,
        storage: &dyn Storage,
        config: PriceConfig,
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for leg in SupportedPairs::<OracleBase>::load(storage)?.query_supported_pairs().into_iter() {
            let path = tree.load_path(&leg.from)?;
            // we need to gather all available prices without NoPrice error
            if let Ok(price) = Self::MARKET_PRICE.price(storage, config, path) {
                prices.push(price);
            }
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

    let mut batch = Batch::default();
    batch.schedule_execute_wasm_reply_error::<_, Nls>(
        &oracle.config.timealarms_contract,
        timealarms::msg::ExecuteMsg::Notify(),
        None,
        1,
    )?;

    Ok(Response::from(batch))
}

// TODO: separation of price feed and alarms notification
pub fn try_notify_alarms<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    max_count: u32,
) -> Result<Response, ContractError>
where
    OracleBase: Currency,
{
    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config);

    let batch = Batch::default();

    let price_config = Feeders::price_config(storage, &oracle.config, block_time)?;
    // re-calculate the price of these currencies
    let prices = oracle
        .get_all_prices(storage, price_config)?;
    // try notify affected subscribers
    MarketAlarms::try_notify_alarms(storage, batch, &prices, max_count)
}
