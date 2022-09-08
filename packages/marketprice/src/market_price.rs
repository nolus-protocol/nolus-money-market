use std::convert::Infallible;

use crate::feed::{Observation, PriceFeed};
use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;
use finance::coin::Coin;
use finance::currency::{Currency, SymbolOwned};
use finance::duration::Duration;

use thiserror::Error;

use finance::price::{self, Price as FinPrice, PriceDTO};
pub struct QueryConfig {
    price_feed_period: Duration,
    required_feeders_cnt: usize,
}

impl QueryConfig {
    pub fn new(price_feed_period: Duration, required_feeders_cnt: usize) -> Self {
        QueryConfig {
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

    #[error("Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),
    #[error("{0}")]
    FromInfallible(#[from] Infallible),
    #[error("{0}")]
    Finance(#[from] finance::error::Error),
    #[error("Unknown currency")]
    UnknownCurrency {},
}

type DenomResolutionPath = Vec<Observation>;
pub struct PriceFeeds<'m>(Map<'m, (SymbolOwned, SymbolOwned), PriceFeed>);

impl<'m> PriceFeeds<'m> {
    pub const fn new(namespace: &'m str) -> PriceFeeds {
        PriceFeeds(Map::new(namespace))
    }

    pub fn get<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        query: QueryConfig,
    ) -> Result<PriceDTO, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        // let base = &query.denom_pair.0;
        // let quote = &query.denom_pair.1;

        // FIXME return PriceDTO::one
        if C::SYMBOL.to_string().eq(&QuoteC::SYMBOL.to_string()) {
            let price: FinPrice<C, QuoteC> =
                price::total_of(Coin::<C>::new(1)).is(Coin::<QuoteC>::new(1));

            return Ok(PriceDTO::try_from(price)?);
        }

        // check if the second part of the pair exists in the storage
        let result: StdResult<Vec<_>> = self
            .0
            .keys(storage, None, None, Order::Descending)
            .collect();
        if !result?.iter().any(|key| key.1.eq(QuoteC::SYMBOL)) {
            return Err(PriceFeedsError::NoPrice {});
        }

        // find a path from Denom 1 to Denom 2
        let mut resolution_path = DenomResolutionPath::new();
        let result = self.find_price::<C, QuoteC>(
            storage,
            current_block_time,
            query.price_feed_period,
            query.required_feeders_cnt,
            &mut resolution_path,
        )?;
        resolution_path.push(result);
        resolution_path.reverse();
        println!("Resolution path {:?}", resolution_path);
        PriceFeeds::calculate_price::<C, QuoteC>(&mut resolution_path)
    }

    pub fn find_price<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        price_feed_period: Duration,
        required_feeders_cnt: usize,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Observation, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        // check for exact match for the denom pair
        let res = self
            .0
            .load(storage, (C::SYMBOL.to_string(), QuoteC::SYMBOL.to_string()));

        match res {
            Ok(last_feed) => {
                // there is a price record for denom pair base to denom pair quote => return price
                let price = last_feed.get_price(
                    current_block_time,
                    price_feed_period,
                    required_feeders_cnt,
                )?;
                Ok(price)
            }
            Err(err) => {
                println!(
                    "No price record for [ {}, {} ]: Error {:?}",
                    C::SYMBOL,
                    QuoteC::SYMBOL,
                    err
                );
                // Try to find transitive path
                if let Ok(Some(q)) = self.search_for_path::<C, QuoteC>(
                    storage,
                    current_block_time,
                    price_feed_period,
                    required_feeders_cnt,
                    resolution_path,
                ) {
                    let observation = q.1.get_price(current_block_time, price_feed_period, 1)?;
                    let price = observation.price();
                    assert_eq!(C::SYMBOL, price.base().symbol());
                    assert_eq!(q.0, price.quote().symbol().to_owned());
                    return Ok(observation);
                }
                Err(PriceFeedsError::NoPrice {})
            }
        }
    }

    fn search_for_path<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        price_feed_period: Duration,
        required_feeders_cnt: usize,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Option<(String, PriceFeed)>, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        // get all entries with key denom pair that stars with the base denom
        let quotes: StdResult<Vec<_>> = self
            .0
            .prefix(C::SYMBOL.to_string())
            .range(storage, None, None, Order::Ascending)
            .collect();

        for current_quote in quotes? {
            if let Ok(observation) = self.find_price::<C, QuoteC>(
                storage,
                current_block_time,
                price_feed_period,
                required_feeders_cnt,
                resolution_path,
            ) {
                resolution_path.push(observation);
                return Ok(Some(current_quote));
            };
        }
        Ok(None)
    }

    // TODO refactore logic
    fn calculate_price<C, QuoteC>(
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<PriceDTO, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        if resolution_path.len() == 1 {
            match resolution_path.first() {
                Some(o) => Ok(o.price()),
                None => Err(PriceFeedsError::NoPrice {}),
            }
        } else {
            let base = C::SYMBOL;
            let mut i = 0;
            assert!(resolution_path[0]
                .price()
                .base()
                .symbol()
                .to_string()
                .eq(&base));
            let first = resolution_path[0].price();
            let mut result = first.clone();

            while !resolution_path.is_empty() {
                if resolution_path[i]
                    .price()
                    .base()
                    .symbol()
                    .to_string()
                    .eq(&base)
                {
                    let val = resolution_path.remove(i);
                    result = val.price();
                    assert_eq!(result.base().symbol().to_string(), base);
                } else {
                    i += 1;
                }
            }
            Ok(result)
        }
    }

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        current_block_time: Timestamp,
        sender_raw: &Addr,
        prices: Vec<PriceDTO>,
        price_feed_period: Duration,
    ) -> Result<(), PriceFeedsError> {
        for price_dto in prices {
            let update_market_price = |old: Option<PriceFeed>| -> StdResult<PriceFeed> {
                let new_feed =
                    Observation::new(sender_raw.clone(), current_block_time, price_dto.clone());
                match old {
                    Some(mut feed) => {
                        feed.update(new_feed, price_feed_period);
                        Ok(feed)
                    }
                    None => Ok(PriceFeed::new(new_feed)),
                }
            };

            self.0.update(
                storage,
                (
                    price_dto.base().symbol().to_string(),
                    price_dto.quote().symbol().to_string(),
                ),
                update_market_price,
            )?;
        }

        Ok(())
    }
}
