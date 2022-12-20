use finance::{
    currency::{self, AnyVisitor, Currency, Symbol, SymbolOwned},
    duration::Duration,
    price::{
        dto::{with_price, WithPrice},
        Price,
    },
};
use sdk::{
    cosmwasm_std::{Addr, Storage, Timestamp},
    cw_storage_plus::Map,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::PriceFeedsError, feed::PriceFeed, CurrencyGroup, SpotPrice};

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
        debug_assert!(Timestamp::default() + price_feed_period <= block_time);
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

    pub fn price<'a, QuoteC, Iter>(
        &'m self,
        storage: &'a dyn Storage,
        config: &'a Config,
        leaf_to_root: Iter,
    ) -> Result<SpotPrice, PriceFeedsError>
    where
        'm: 'a,
        QuoteC: Currency + DeserializeOwned,
        Iter: Iterator<Item = Symbol<'a>> + DoubleEndedIterator,
    {
        let mut root_to_leaf = leaf_to_root.rev();
        let _root = root_to_leaf.next();
        debug_assert_eq!(Some(QuoteC::TICKER), _root);
        PriceCollect::do_collect(
            root_to_leaf,
            self,
            storage,
            config,
            Price::<QuoteC, QuoteC>::identity(),
        )
    }

    fn price_of_feed<C, QuoteC>(
        &self,
        storage: &dyn Storage,
        config: &Config,
    ) -> Result<Price<C, QuoteC>, PriceFeedsError>
    where
        C: Currency + DeserializeOwned,
        QuoteC: Currency + DeserializeOwned,
    {
        let feed_bin = self
            .0
            .may_load(storage, (C::TICKER.into(), QuoteC::TICKER.into()))?;
        load_feed(feed_bin).and_then(|feed| feed.calc_price(config))
    }
}

fn load_feed<BaseC, QuoteC>(
    feed_bin: Option<PriceFeedBin>,
) -> Result<PriceFeed<BaseC, QuoteC>, PriceFeedsError>
where
    BaseC: Currency + DeserializeOwned,
    QuoteC: Currency + DeserializeOwned,
{
    feed_bin.map_or_else(
        || Ok(PriceFeed::<BaseC, QuoteC>::default()),
        |bin| postcard::from_bytes(&bin).map_err(Into::into),
    )
}
struct PriceCollect<'a, Iter, BaseC, QuoteC>
where
    Iter: Iterator<Item = Symbol<'a>>,
    BaseC: Currency,
    QuoteC: Currency,
{
    currency_path: Iter,
    feeds: &'a PriceFeeds<'a>,
    storage: &'a dyn Storage,
    config: &'a Config,
    price: Price<BaseC, QuoteC>,
}
impl<'a, Iter, BaseC, QuoteC> PriceCollect<'a, Iter, BaseC, QuoteC>
where
    Iter: Iterator<Item = Symbol<'a>>,
    BaseC: Currency + DeserializeOwned,
    QuoteC: Currency,
{
    fn do_collect(
        mut currency_path: Iter,
        feeds: &'a PriceFeeds<'a>,
        storage: &'a dyn Storage,
        config: &'a Config,
        price: Price<BaseC, QuoteC>,
    ) -> Result<SpotPrice, PriceFeedsError> {
        if let Some(next_currency) = currency_path.next() {
            let next_collect = PriceCollect {
                currency_path,
                feeds,
                storage,
                config,
                price,
            };
            currency::visit_any_on_ticker::<CurrencyGroup, _>(next_currency, next_collect)
        } else {
            Ok(price.into())
        }
    }
}
impl<'a, Iter, QuoteC, QuoteQuoteC> AnyVisitor for PriceCollect<'a, Iter, QuoteC, QuoteQuoteC>
where
    Iter: Iterator<Item = Symbol<'a>>,
    QuoteC: Currency + DeserializeOwned,
    QuoteQuoteC: Currency,
{
    type Output = SpotPrice;
    type Error = PriceFeedsError;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        let next_price = self
            .feeds
            .price_of_feed::<C, _>(self.storage, self.config)?;
        let total_price = next_price * self.price;
        PriceCollect::do_collect(
            self.currency_path,
            self.feeds,
            self.storage,
            self.config,
            total_price,
        )
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
        feed_bin: Option<PriceFeedBin>,
        from: &'a Addr,
        at: Timestamp,
        validity: Duration,
    }

    impl<'a> WithPrice for AddObservation<'a> {
        type Output = PriceFeedBin;
        type Error = PriceFeedsError;

        fn exec<C, QuoteC>(self, price: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
        where
            C: Currency + Serialize + DeserializeOwned,
            QuoteC: Currency + Serialize + DeserializeOwned,
        {
            load_feed(self.feed_bin).and_then(|feed| {
                let feed = feed.add_observation(self.from.clone(), self.at, price, self.validity);
                postcard::to_allocvec(&feed).map_err(Into::into)
            })
        }
    }
    with_price::execute(
        price,
        AddObservation {
            feed_bin,
            from,
            at,
            validity,
        },
    )
}

#[cfg(test)]
mod test {
    use currency::{
        lease::{Atom, Cro, Osmo, Stars, Wbtc},
        lpn::Usdc,
    };
    use finance::{
        coin::Coin,
        currency::Currency,
        duration::Duration,
        price::{self, Price},
    };
    use sdk::cosmwasm_std::{Addr, MemoryStorage, Timestamp};

    use crate::{error::PriceFeedsError, market_price::Config, SpotPrice};

    use super::PriceFeeds;

    const FEEDS_NAMESPACE: &str = "feeds";
    const REQUIRED_FEEDERS_CNT: usize = 1;
    const FEEDER: &str = "0xifeege";
    const FEED_MAX_AGE: Duration = Duration::from_secs(30);
    const NOW: Timestamp = Timestamp::from_seconds(FEED_MAX_AGE.secs() * 2);

    #[test]
    fn no_feed() {
        let feeds = PriceFeeds::new(FEEDS_NAMESPACE);
        let storage = MemoryStorage::new();

        assert_eq!(
            Ok(Price::<Atom, Atom>::identity().into()),
            feeds.price::<Atom, _>(&storage, &config(), [Atom::TICKER].into_iter())
        );

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<Atom, _>(
                &storage,
                &config(),
                [Wbtc::TICKER, Atom::TICKER].into_iter()
            )
        );
    }

    #[test]
    fn feed_pair() {
        let feeds = PriceFeeds::new(FEEDS_NAMESPACE);
        let mut storage = MemoryStorage::new();
        let new_price: SpotPrice = price::total_of(Coin::<Wbtc>::new(1))
            .is(Coin::<Usdc>::new(18500))
            .into();

        feeds
            .feed(
                &mut storage,
                NOW,
                &Addr::unchecked(FEEDER),
                &[new_price.clone()],
                FEED_MAX_AGE,
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<Atom, _>(
                &storage,
                &config(),
                [Wbtc::TICKER, Atom::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok(new_price),
            feeds.price::<Usdc, _>(
                &storage,
                &config(),
                [Wbtc::TICKER, Usdc::TICKER].into_iter()
            )
        );
    }

    #[test]
    fn feed_pairs() {
        let feeds = PriceFeeds::new(FEEDS_NAMESPACE);
        let mut storage = MemoryStorage::new();
        let config = config();
        let new_price12 = price::total_of(Coin::<Wbtc>::new(1)).is(Coin::<Osmo>::new(2));
        let new_price23 = price::total_of(Coin::<Osmo>::new(1)).is(Coin::<Usdc>::new(3));
        let new_price24 = price::total_of(Coin::<Osmo>::new(1)).is(Coin::<Stars>::new(4));

        feeds
            .feed(
                &mut storage,
                NOW,
                &Addr::unchecked(FEEDER),
                &[new_price24.into(), new_price12.into(), new_price23.into()],
                FEED_MAX_AGE,
            )
            .unwrap();

        assert_eq!(
            Err(PriceFeedsError::NoPrice()),
            feeds.price::<Cro, _>(&storage, &config, [Wbtc::TICKER, Cro::TICKER].into_iter())
        );
        assert_eq!(
            Ok(new_price12.into()),
            feeds.price::<Osmo, _>(&storage, &config, [Wbtc::TICKER, Osmo::TICKER].into_iter())
        );
        assert_eq!(
            Ok(new_price23.into()),
            feeds.price::<Usdc, _>(&storage, &config, [Osmo::TICKER, Usdc::TICKER].into_iter())
        );
        assert_eq!(
            Ok(new_price24.into()),
            feeds.price::<Stars, _>(&storage, &config, [Osmo::TICKER, Stars::TICKER].into_iter())
        );
        assert_eq!(
            Ok((new_price12 * new_price23).into()),
            feeds.price::<Usdc, _>(
                &storage,
                &config,
                [Wbtc::TICKER, Osmo::TICKER, Usdc::TICKER].into_iter()
            )
        );
        assert_eq!(
            Ok((new_price12 * new_price24).into()),
            feeds.price::<Stars, _>(
                &storage,
                &config,
                [Wbtc::TICKER, Osmo::TICKER, Stars::TICKER].into_iter()
            )
        );
    }

    fn config() -> Config {
        Config::new(FEED_MAX_AGE, REQUIRED_FEEDERS_CNT, NOW)
    }
}
