use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Feeds<OracleBase> {
    config: Config,
    _base: PhantomData<OracleBase>,
}

impl<OracleBase> Feeds<OracleBase>
where
    OracleBase: Currency,
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
        config: PriceConfig,
        currencies: &[SymbolOwned],
    ) -> Result<Vec<SpotPrice>, ContractError> {
        let tree: SupportedPairs<OracleBase> = SupportedPairs::load(storage)?;
        let mut prices = vec![];
        for currency in currencies {
            let path = tree.load_path(currency)?;
            let price = Self::MARKET_PRICE.price(storage, config, &path)?;
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
    OracleBase: Currency,
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
/*
pub fn try_notify_alarms<OracleBase>(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    prices: Vec<SpotPrice>,
) -> Result<Response, ContractError>
where
    OracleBase: Currency,
{
    let config = Config::load(storage)?;
    let oracle = Feeds::<OracleBase>::with(config);
    let supported_pairs = SupportedPairs::<OracleBase>::load(storage)?;

    let mut batch = Batch::default();

    let affected = prices
        .into_iter()
        .map(|price| supported_pairs.load_affected((price.base().ticker(), price.quote().ticker())))
        .try_fold(vec![], |mut acc, el| -> Result<_, ContractError> {
            acc.extend(el?);
            Ok(acc)
        })?;

    if !affected.is_empty() {
        let parameters = Feeders::query_config(storage, &oracle.config, block_time)?;
        // re-calculate the price of these currencies
        let updated_prices = oracle
            .get_prices(storage, parameters, &affected)?;
        // try notify affected subscribers
        MarketAlarms::try_notify_alarms(storage, updated_prices, &mut batch)?;
    }
    Ok(Response::from(batch))
}
*/
