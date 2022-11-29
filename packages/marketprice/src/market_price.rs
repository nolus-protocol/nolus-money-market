use currency::payment::PaymentGroup;
use finance::{currency::SymbolOwned, duration::Duration};
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage, Timestamp},
    cw_storage_plus::Map,
};

use crate::{error::PriceFeedsError, feed::PriceFeed, SpotPrice};

#[derive(Clone, Copy, Debug)]
pub struct Config {
    price_feed_period: Duration,
    required_feeders_cnt: usize,
    block_time: Timestamp,
}

impl Config {
    pub fn new(
        price_feed_period: Duration,
        required_feeders_cnt: usize,
        block_time: Timestamp,
    ) -> Self {
        Config {
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

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        current_block_time: Timestamp,
        sender_raw: &Addr,
        mut prices: Vec<SpotPrice>,
        price_feed_period: Duration,
    ) -> Result<(), PriceFeedsError> {
        while let Some(price_dto) = prices.pop() {
            self.0.update(
                storage,
                (
                    price_dto.base().ticker().to_string(),
                    price_dto.quote().ticker().to_string(),
                ),
                |old: Option<PriceFeed>| -> StdResult<PriceFeed> {
                    Ok(old.unwrap_or_default().add_observation(
                        sender_raw.clone(),
                        current_block_time,
                        price_dto,
                        price_feed_period,
                    ))
                },
            )?;
        }

        Ok(())
    }

    pub fn price(
        &self,
        storage: &dyn Storage,
        config: Config,
        path: Vec<SymbolOwned>,
    ) -> Result<SpotPrice, PriceFeedsError> {
        let mut resolution_path = DenomResolutionPath::new();

        if let Some((first, elements)) = path.split_first() {
            let mut base = first;
            for quote in elements {
                let price_dto =
                    self.price_of_feed(storage, base.to_string(), quote.to_string(), config)?;
                base = quote;
                //TODO multiply immediatelly than collecting in a vector and then PriceFeeds::calculate_price
                resolution_path.push(price_dto);
            }
        }
        PriceFeeds::calculate_price(&resolution_path)
    }

    fn price_of_feed(
        &self,
        storage: &dyn Storage,
        base: SymbolOwned,
        quote: SymbolOwned,
        config: Config,
    ) -> Result<SpotPrice, PriceFeedsError> {
        match self.0.may_load(storage, (base, quote))? {
            Some(feed) => Ok(feed.get_price(config)?),
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
}
