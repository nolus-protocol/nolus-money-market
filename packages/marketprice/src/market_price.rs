use std::convert::{Infallible, TryFrom, TryInto};

use crate::feed::{Observation, PriceFeed};
use crate::storage::DenomPair;
use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;
use finance::currency::Currency;
use finance::duration::Duration;

use thiserror::Error;

use finance::coin::{Coin as FinCoin, CoinDTO};
use finance::price::{self, Price as FinPrice, PriceDTO};

pub struct PriceQuery {
    denom_pair: DenomPair,
    price_feed_period: Duration,
    required_feeders_cnt: usize,
}

impl PriceQuery {
    pub fn new(
        denom_pair: DenomPair,
        price_feed_period: Duration,
        required_feeders_cnt: usize,
    ) -> Self {
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

    #[error("Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),
    #[error("{0}")]
    FromInfallible(#[from] Infallible),
    #[error("{0}")]
    Finance(#[from] finance::error::Error),
}

type DenomResolutionPath = Vec<Observation>;
pub struct PriceFeeds<'m>(Map<'m, DenomPair, PriceFeed>);

impl<'m> PriceFeeds<'m> {
    pub const fn new(namespace: &'m str) -> PriceFeeds {
        PriceFeeds(Map::new(namespace))
    }

    // FIXME: use generics to set <C, QuoteC> and replace denom_pair
    fn get(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        query: PriceQuery,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let base = &query.denom_pair.0;
        let quote = &query.denom_pair.1;

        // FIXME return PriceDTO::one
        // if base.eq(quote) {
        //     let price: FinPrice<C, QuoteC> = price::total_of(FinCoin::new(1)).is(FinCoin::new(1));

        //     return Ok(PriceDTO::try_from(price)?);
        // }

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
        resolution_path.push(result);
        resolution_path.reverse();
        println!("Resolution path {:?}", resolution_path);
        PriceFeeds::calculate_price(query.denom_pair, &mut resolution_path)
    }

    //TODO remove
    pub fn get_converted_dto_price(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        query: PriceQuery,
    ) -> Result<PriceDTO, PriceFeedsError> {
        self.get(storage, current_block_time, query)
    }

    pub fn get_converted_price<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        query: PriceQuery,
    ) -> Result<FinPrice<C, QuoteC>, PriceFeedsError>
    where
        C: 'static + Currency,
        QuoteC: 'static + Currency,
    {
        let calculated_price = self.get(storage, current_block_time, query)?;
        Ok(calculated_price.try_into()?)
    }

    pub fn find_price_for_pair(
        &self,
        storage: &dyn Storage,
        current_block_time: Timestamp,
        denom_pair: DenomPair,
        price_feed_period: Duration,
        required_feeders_cnt: usize,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Observation, PriceFeedsError> {
        // check for exact match for the denom pair
        let res = self.0.load(storage, denom_pair.clone());

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
                    let observation =
                        q.1.get_price(current_block_time, Duration::from_secs(60), 1)?;
                    let price = observation.price();
                    assert_eq!(denom_pair.0, price.base().symbol().to_owned());
                    assert_eq!(q.0, price.quote().symbol().to_owned());
                    return Ok(observation);
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
        price_feed_period: Duration,
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
            if let Ok(observation) = self.find_price_for_pair(
                storage,
                current_block_time,
                (current_quote.0.clone(), searched_quote.clone()),
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
    fn calculate_price(
        denom_pair: DenomPair,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<PriceDTO, PriceFeedsError> {
        if resolution_path.len() == 1 {
            match resolution_path.first() {
                Some(o) => Ok(o.price()),
                None => Err(PriceFeedsError::NoPrice {}),
            }
        } else {
            let mut base = denom_pair.0;
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

    fn convert_to_price<C, QuoteC>(
        first: CoinDTO,
        second: CoinDTO,
    ) -> Result<FinPrice<C, QuoteC>, PriceFeedsError>
    where
        C: 'static + Currency,
        QuoteC: 'static + Currency,
    {
        let c_first = FinCoin::<C>::try_from(first)?;
        let c_second = FinCoin::<QuoteC>::try_from(second)?;
        Ok(price::total_of(c_first).is(c_second))
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
