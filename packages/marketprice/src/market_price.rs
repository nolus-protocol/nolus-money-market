use crate::error::PriceFeedsError;
use crate::feed::{Observation, PriceFeed};
use crate::Multiply;
use cosmwasm_std::{Addr, StdResult, Storage, Timestamp};
use currency::payment::PaymentGroup;
use cw_storage_plus::Map;

use finance::currency::SymbolOwned;
use finance::duration::Duration;

use finance::price::dto::with_price;
use finance::price::dto::PriceDTO;

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

    pub fn price(
        &self,
        storage: &dyn Storage,
        parameters: Parameters,
        path: Vec<SymbolOwned>,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let mut resolution_path = DenomResolutionPath::new();

        if let Some((first, elements)) = path.split_first() {
            let mut base = first;
            for quote in elements {
                let price_dto =
                    match self.load(storage, base.to_string(), quote.to_string(), parameters) {
                        Ok(price) => price,
                        Err(err) => {
                            return Err(err);
                        }
                    };
                base = quote;
                resolution_path.push(price_dto);
            }
        }
        PriceFeeds::calculate_price(&mut resolution_path)
    }

    pub fn load(
        &self,
        storage: &dyn Storage,
        base: SymbolOwned,
        quote: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError> {
        match self.0.may_load(storage, (base, quote))? {
            Some(feed) => Ok(feed.get_price(parameters)?.price()),
            None => Err(PriceFeedsError::NoPrice()),
        }
    }
    // TODO remove move price calculation to the finance library
    fn calculate_price(
        resolution_path: &mut DenomResolutionPath,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let mut first = match resolution_path.first() {
            Some(price) => Ok(price.to_owned()),
            None => Err(PriceFeedsError::NoPrice {}),
        }?;

        if resolution_path.len() == 1 {
            return Ok(first);
        }
        resolution_path.remove(0);
        for p in resolution_path.iter() {
            first = with_price::execute::<PaymentGroup, Multiply>(
                first.clone(),
                Multiply::with(p.to_owned()),
            )?;
        }

        Ok(first)
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
