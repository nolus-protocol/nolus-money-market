use currency::payment::PaymentGroup;
use finance::{currency::SymbolOwned, duration::Duration};
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage, Timestamp},
    cw_storage_plus::Map,
};

use crate::{
    error::PriceFeedsError,
    feed::{Observation, PriceFeed},
    SpotPrice,
};

#[derive(Clone, Copy, Debug)]
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

type DenomResolutionPath = Vec<SpotPrice>;
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
    ) -> Result<SpotPrice, PriceFeedsError> {
        let mut resolution_path = DenomResolutionPath::new();

        if let Some((first, elements)) = path.split_first() {
            let mut base = first;
            for quote in elements {
                let price_dto =
                    self.load(storage, base.to_string(), quote.to_string(), parameters)?;
                base = quote;
                resolution_path.push(price_dto);
            }
        }
        PriceFeeds::calculate_price(&resolution_path)
    }

    pub fn load(
        &self,
        storage: &dyn Storage,
        base: SymbolOwned,
        quote: SymbolOwned,
        parameters: Parameters,
    ) -> Result<SpotPrice, PriceFeedsError> {
        match self.0.may_load(storage, (base, quote))? {
            Some(feed) => Ok(feed.get_price(parameters)?.price()),
            None => Err(PriceFeedsError::NoPrice()),
        }
    }

    fn calculate_price(
        resolution_path: &DenomResolutionPath,
    ) -> Result<SpotPrice, PriceFeedsError> {
        if let Some((first, rest)) = resolution_path.split_first() {
            rest.iter()
                .fold(Ok(first.to_owned()), |result_c1, c2| {
                    result_c1.and_then(|c1| c1.multiply::<PaymentGroup>(c2))
                })
                .map_err(|e| e.into())
        } else {
            Err(PriceFeedsError::NoPrice {})
        }
    }

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        current_block_time: Timestamp,
        sender_raw: &Addr,
        prices: &[SpotPrice],
        price_feed_period: Duration,
    ) -> Result<(), PriceFeedsError> {
        for price_dto in prices {
            self.0.update(
                storage,
                (
                    price_dto.base().ticker().to_string(),
                    price_dto.quote().ticker().to_string(),
                ),
                |old: Option<PriceFeed>| -> StdResult<PriceFeed> {
                    let new_feed = Observation::new(
                        sender_raw.clone(),
                        current_block_time,
                        price_dto.to_owned(),
                    );

                    if let Some(mut feed) = old {
                        feed.update(new_feed, price_feed_period);
                        Ok(feed)
                    } else {
                        Ok(PriceFeed::new(new_feed))
                    }
                },
            )?;
        }

        Ok(())
    }
}
