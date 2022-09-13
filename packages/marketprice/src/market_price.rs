use std::convert::TryFrom;

use crate::error::PriceFeedsError;
use crate::feed::{Observation, PriceFeed};
use crate::WithQuote;
use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage, Timestamp};
use cw_storage_plus::Map;
use finance::coin::Coin;
use finance::currency::{Currency, SymbolOwned};
use finance::duration::Duration;

use finance::price::{self, PriceDTO};

#[derive(Clone, Copy)]
pub struct Parameters {
    price_feed_period: Duration,
    required_feeders_cnt: usize,
    block_time: Timestamp,
}

impl Parameters {
    pub fn new(
        price_feed_period: Duration,
        required_feeders_cnt: usize,
        block_time: Timestamp,
    ) -> Self {
        Parameters {
            price_feed_period,
            required_feeders_cnt,
            block_time,
        }
    }
    pub fn block_time(&self) -> Timestamp {
        self.block_time
    }
    pub fn feeders(&self) -> usize {
        self.required_feeders_cnt
    }
    pub fn period(&self) -> Duration {
        self.price_feed_period
    }
}

type DenomResolutionPath = Vec<PriceDTO>;
pub struct PriceFeeds<'m>(Map<'m, (SymbolOwned, SymbolOwned), PriceFeed>);

impl<'m> PriceFeeds<'m> {
    pub const fn new(namespace: &'m str) -> PriceFeeds {
        PriceFeeds(Map::new(namespace))
    }

    //TODO REMOVE
    pub fn get<C, QuoteC>(
        &self,
        _storage: &dyn Storage,
        _parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError>
    where
        C: Currency,
        QuoteC: Currency,
    {
        Ok(PriceDTO::try_from(
            price::total_of(Coin::<C>::new(1)).is(Coin::<QuoteC>::new(1)),
        )?)
    }

    pub fn price(
        &self,
        storage: &dyn Storage,
        parameters: Parameters,
        base: SymbolOwned,
        quote: SymbolOwned,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let mut resolution_path = DenomResolutionPath::new();

        let res = self.price_impl(storage, parameters, &base, quote, resolution_path.as_mut())?;
        // resolution_path.push(res);
        // resolution_path.reverse();

        // PriceFeeds::calculate_price(base, &mut resolution_path)
        Ok(res)
    }

    fn price_impl(
        &self,
        storage: &dyn Storage,
        parameters: Parameters,
        base: &SymbolOwned,
        quote: SymbolOwned,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let price_dto = match WithQuote::cmd(storage, base.to_owned(), quote.clone(), parameters) {
            Ok(price) => price,
            Err(PriceFeedsError::NoPrice()) => {
                if let Some(feed) = self.search_for_path(
                    storage,
                    parameters,
                    base.to_owned(),
                    quote,
                    resolution_path,
                )? {
                    return Ok(feed.get_price(parameters)?.price());
                }
                return Err(PriceFeedsError::NoPrice {});
            }
            Err(_err) => {
                return Err(PriceFeedsError::NoPrice {});
            }
        };

        Ok(price_dto)
    }

    pub fn load(
        &self,
        storage: &dyn Storage,
        base: SymbolOwned,
        quote: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError> {
        Ok(self
            .0
            .load(storage, (base, quote))?
            .get_price(parameters)?
            .price())
    }

    fn search_for_path(
        &self,
        storage: &dyn Storage,
        parameters: Parameters,
        base: SymbolOwned,
        quote: SymbolOwned,
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<Option<PriceFeed>, PriceFeedsError> {
        // get all entries with key denom pair that stars with the base denom
        let quotes: Vec<_> = self
            .0
            .prefix(base.clone())
            .range(storage, None, None, Order::Ascending)
            .filter_map(|res| res.ok())
            .collect();

        println!("========{}===={}===========", base, quote.clone());

        for (current_quote, feed) in quotes {
            if let Ok(price) = self.price_impl(
                storage,
                parameters,
                &current_quote,
                quote.clone(),
                resolution_path,
            ) {
                resolution_path.push(price.clone());
                return Ok(Some(feed));
            };
        }
        Ok(None)
    }

    // TODO remove move price calculation to the finance library
    // fn calculate_price(
    //     base: SymbolOwned,
    //     resolution_path: &mut DenomResolutionPath,
    // ) -> Result<PriceDTO, PriceFeedsError> {
    //     if resolution_path.len() == 1 {
    //         match resolution_path.first() {
    //             Some(price) => Ok(price.to_owned()),
    //             None => Err(PriceFeedsError::NoPrice {}),
    //         }
    //     } else {
    //         let mut i = 0;
    //         assert!(resolution_path[0].base().symbol().to_string().eq(&base));
    //         let first = resolution_path[0].base().clone();
    //         let mut result = first.clone();

    //         while !resolution_path.is_empty() {
    //             if resolution_path[i].base().symbol().to_string().eq(&base) {
    //                 let price = resolution_path.remove(i);
    //                 base = price.quote().symbol().to_string();

    //                 let x = price.into();

    //                 result = price.total(&result);
    //                 // assert_eq!(result.base().symbol().to_string(), base);
    //             } else {
    //                 i += 1;
    //             }
    //         }
    //         Ok(first)
    //     }
    // }

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

fn print_no_price(base: &str, quote: &str, err: StdError) {
    println!(
        "No price record for [ {}, {} ]: Error {:?}",
        base, quote, err
    );
}
