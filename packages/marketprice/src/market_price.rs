use currency::payment::PaymentGroup;
use finance::{
    currency::{Currency, Symbol, SymbolOwned},
    duration::Duration,
};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
};
use serde::Serialize;

use crate::{
    error::PriceFeedsError,
    feed::PriceFeed,
    with_feed::{self, WithPriceFeed},
    CurrencyGroup, SpotPrice,
};

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
pub type PriceFeedBin = Vec<u8>;
pub struct PriceFeeds<'m>(Map<'m, (SymbolOwned, SymbolOwned), PriceFeedBin>);

impl<'m> PriceFeeds<'m> {
    pub const fn new(namespace: &'m str) -> PriceFeeds {
        PriceFeeds(Map::new(namespace))
    }

    pub fn feed(
        &self,
        storage: &mut dyn Storage,
        current_block_time: Timestamp,
        sender_raw: &Addr,
        prices: &[SpotPrice],
        price_feed_period: Duration,
    ) -> Result<(), PriceFeedsError> {
        for price in prices {
            self.0.update(
                storage,
                (
                    price.base().ticker().to_string(),
                    price.quote().ticker().to_string(),
                ),
                |feed: Option<PriceFeedBin>| -> Result<PriceFeedBin, PriceFeedsError> {
                    add_observation(
                        feed,
                        sender_raw,
                        current_block_time,
                        price,
                        price_feed_period,
                    )
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
                let price_dto = self.price_of_feed(storage, base, quote, config)?;
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
        base: Symbol,
        quote: Symbol,
        config: Config,
    ) -> Result<SpotPrice, PriceFeedsError> {
        struct CalculatePrice(Config);
        impl WithPriceFeed for CalculatePrice {
            type Output = SpotPrice;
            type Error = PriceFeedsError;

            fn exec<C, QuoteC>(
                self,
                feed: PriceFeed<C, QuoteC>,
            ) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
                QuoteC: Currency,
            {
                feed.get_price(self.0).map(Into::into)
            }
        }
        let feed_bin = self.0.may_load(storage, (base.into(), quote.into()))?;
        with_feed::execute::<CurrencyGroup, CurrencyGroup, _>(
            base,
            quote,
            feed_bin,
            CalculatePrice(config),
        )
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

fn add_observation(
    feed_bin: Option<PriceFeedBin>,
    from: &Addr,
    at: Timestamp,
    price: &SpotPrice,
    validity: Duration,
) -> Result<PriceFeedBin, PriceFeedsError> {
    struct AddObservation<'a> {
        from: &'a Addr,
        at: Timestamp,
        price: &'a SpotPrice,
        validity: Duration,
    }

    impl<'a> WithPriceFeed for AddObservation<'a> {
        type Output = PriceFeedBin;
        type Error = PriceFeedsError;

        fn exec<C, QuoteC>(self, feed: PriceFeed<C, QuoteC>) -> Result<Self::Output, Self::Error>
        where
            C: Currency + Serialize,
            QuoteC: Currency + Serialize,
        {
            let feed = feed.add_observation(
                self.from.clone(),
                self.at,
                self.price.try_into()?,
                self.validity,
            );
            rmp_serde::to_vec(&feed).map_err(Into::into)
        }
    }
    with_feed::execute::<CurrencyGroup, CurrencyGroup, _>(
        price.base().ticker(),
        price.quote().ticker(),
        feed_bin,
        AddObservation {
            from,
            at,
            price,
            validity,
        },
    )
}
