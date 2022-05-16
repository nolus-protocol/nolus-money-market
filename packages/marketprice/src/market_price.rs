use std::ops::Mul;

use cosmwasm_std::{Addr, Decimal256, Order, StdError, StdResult, Storage, Timestamp};
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
    pub fn new(denom_pair: DenomPair, price_feed_period: u64, required_feeders_cnt: usize) -> Self {
        PriceQuery {
            denom_pair,
            price_feed_period,
            required_feeders_cnt,
        }
    }
}

pub struct DenomPairPrice {
    pub pair: DenomPair,
    pub price: Observation,
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

type DenomResolutionPath = Vec<(DenomPair, Observation)>;
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
    ) -> Result<Decimal256, PriceFeedsError> {
        let base = &query.denom_pair.0;
        let quote = &query.denom_pair.1;

        if base.eq(quote) {
            return Ok(Decimal256::one());
        }

        // check if the second part of the pair exists in the storage
        let result: StdResult<Vec<_>> = self
            .0
            .keys(storage, None, None, Order::Descending)
            .collect();
        if !result?.iter().any(|key| key.1.eq(quote)) {
            return Err(PriceFeedsError::NoPrice {});
        }

        // find a path from Denom 1 to Denom 2
        let mut resolution_path = DenomResolutionPath::new();
        let result = self.find_price_for_pair(
            storage,
            current_block_time,
            query.denom_pair.clone(),
            query.price_feed_period,
            query.required_feeders_cnt,
            &mut resolution_path,
        )?;
        resolution_path.push((result.pair, result.price));
        resolution_path.reverse();
        println!("Resolution path {:?}", resolution_path);
        PriceFeeds::calculate_price(query.denom_pair, &mut resolution_path)
    }

    pub fn find_price_for_pair(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        denom_pair: DenomPair,
        price_feed_period: u64,
        required_feeders_cnt: usize,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<DenomPairPrice, PriceFeedsError> {
        // check for exact match for the denom pair
        let res = self.0.load(storage, denom_pair.clone());

        match res {
            Ok(last_feed) => {
                // there is a price record for denom pair base to denom pair quote => return price
                let price = last_feed.get_price(
                    current_block_time,
                    price_feed_period,
                    required_feeders_cnt,
                );
                Ok(DenomPairPrice {
                    pair: denom_pair,
                    price: price?,
                })
            }
            Err(err) => {
                println!(
                    "No price record for denom pair [ {:?} ]: Error {:?}",
                    denom_pair, err
                );
                // Try to find transitive path
                if let Ok(Some(q)) = self.search_for_path(
                    storage,
                    current_block_time,
                    denom_pair.clone(),
                    price_feed_period,
                    required_feeders_cnt,
                    resolution_path,
                ) {
                    let price = q.1.get_price(current_block_time, 60, 1)?;
                    return Ok(DenomPairPrice {
                        pair: (denom_pair.0, q.0),
                        price,
                    });
                }
                Err(PriceFeedsError::NoPrice {})
            }
        }
    }

    fn search_for_path(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        denom_pair: DenomPair,
        price_feed_period: u64,
        required_feeders_cnt: usize,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Option<(String, PriceFeed)>, PriceFeedsError> {
        let prefix = denom_pair.0;
        let searched_quote = denom_pair.1;
        // get all entries with key denom pair that stars with the base denom
        let quotes: StdResult<Vec<_>> = self
            .0
            .prefix(prefix)
            .range(storage, None, None, Order::Ascending)
            .collect();

        for current_quote in quotes? {
            if let Ok(o) = self.find_price_for_pair(
                storage,
                current_block_time,
                (current_quote.0.clone(), searched_quote.clone()),
                price_feed_period,
                required_feeders_cnt,
                resolution_path,
            ) {
                resolution_path.push((o.pair, o.price));
                return Ok(Some(current_quote));
            };
        }
        Ok(None)
    }

    fn calculate_price(
        denom_pair: DenomPair,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Decimal256, PriceFeedsError> {
        if resolution_path.len() == 1 {
            match resolution_path.first() {
                Some(o) => Ok(o.1.price()),
                None => Err(PriceFeedsError::NoPrice {}),
            }
        } else {
            let mut base = denom_pair.0;
            let mut i = 0;
            let mut price = Decimal256::one();
            while !resolution_path.is_empty() {
                if resolution_path[i].0 .0.eq(&base) {
                    let val = resolution_path.remove(i);
                    base = val.0 .1;
                    price = price.mul(val.1.price());
                } else {
                    i += 1;
                }
            }
            Ok(price)
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
