use cosmwasm_std::{Addr, Decimal256, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::feed::{Denom, DenomPair, Observation, PriceFeed};

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct PriceResponse {
    pub rate: Decimal256,
    pub last_updated_time: Timestamp,
}

pub struct PriceQuery {
    denom_pair: DenomPair,
    price_feed_period: u64,
    required_feeders_cnt: usize,
}
impl PriceQuery {
    pub fn new(
        denom_pair: DenomPair,
        price_feed_period: u64,
        required_feeders_cnt: usize,
    ) -> PriceQuery {
        PriceQuery {
            denom_pair,
            price_feed_period,
            required_feeders_cnt,
        }
    }
}

/// Errors returned from Admin
#[derive(Error, Debug, PartialEq)]
pub enum PriceFeedsError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Given address already registered as a price feeder")]
    FeederAlreadyRegistered {},

    #[error("Given address not registered as a price feeder")]
    FeederNotRegistered {},

    #[error("No price for pair")]
    NoPrice {},
}

// state/logic
pub struct PriceFeeds<'m>(Map<'m, DenomPair, PriceFeed>);

impl<'m> PriceFeeds<'m> {
    pub const fn new(namespace: &'m str) -> PriceFeeds {
        PriceFeeds(Map::new(namespace))
    }

    pub fn get(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        query: PriceQuery,
    ) -> Result<Observation, PriceFeedsError> {
        let res = self
            .0
            .load(storage, (query.denom_pair.0, query.denom_pair.1));
        match res {
            Ok(last_feed) => last_feed.get_price(
                current_block_time,
                query.price_feed_period,
                query.required_feeders_cnt,
            ),
            Err(_) => Err(PriceFeedsError::NoPrice {}),
        }
    }

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        current_block_time: Timestamp,
        sender_raw: Addr,
        base: Denom,
        prices: Vec<(Denom, Decimal256)>,
        price_feed_period: u64,
    ) -> Result<(), PriceFeedsError> {
        for price in prices {
            let quote: String = price.0;
            let price: Decimal256 = price.1;

            let update_market_price = |old: Option<PriceFeed>| -> StdResult<PriceFeed> {
                let new_feed = Observation::new(sender_raw.clone(), current_block_time, price);
                match old {
                    Some(mut feed) => {
                        feed.update(new_feed, price_feed_period);
                        Ok(feed)
                    }
                    None => Ok(PriceFeed::new(new_feed)),
                }
            };

            self.0
                .update(storage, (base.clone(), quote), update_market_price)?;
        }

        Ok(())
    }
}
