use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use currency::{Currency, SymbolOwned};
use finance::price::base::BasePrice;
use marketprice::SpotPrice;
use platform::{
    dispatcher::{AlarmsDispatcher, Id},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};
use swap::SwapGroup;

use crate::{
    contract::{
        alarms::MarketAlarms,
        oracle::feed::{AllPricesIterItem, Feeds},
    },
    error::ContractError,
    msg::{AlarmsStatusResponse, ExecuteAlarmMsg},
    result::ContractResult,
    state::{config::Config, supported_pairs::SupportedPairs},
};

use self::feeder::Feeders;

pub mod feed;
pub mod feeder;

pub(crate) type CalculateAllPricesIterItem<OracleBase> = AllPricesIterItem<OracleBase>;

pub(crate) struct Oracle<'storage, S, OracleBase>
where
    S: Deref<Target = dyn Storage + 'storage>,
    OracleBase: Currency + DeserializeOwned,
{
    storage: S,
    tree: SupportedPairs<OracleBase>,
    feeders: usize,
    feeds: Feeds<OracleBase>,
}

impl<'storage, S, OracleBase> Oracle<'storage, S, OracleBase>
where
    S: Deref<Target = dyn Storage + 'storage>,
    OracleBase: Currency + DeserializeOwned,
{
    pub fn load(storage: S) -> Result<Self, ContractError> {
        let tree = SupportedPairs::load(storage.deref())?;
        let feeders = Feeders::total_registered(storage.deref())?;
        let config = Config::load(storage.deref())?;
        let feeds = Feeds::<OracleBase>::with(config.price_config);

        Ok(Self {
            storage,
            tree,
            feeders,
            feeds,
        })
    }

    pub(super) fn try_query_alarms(
        &self,
        block_time: Timestamp,
    ) -> Result<AlarmsStatusResponse, ContractError> {
        MarketAlarms::new(self.storage.deref())
            .try_query_alarms::<_, OracleBase>(self.calc_all_prices(block_time))
            .map(|remaining_alarms| AlarmsStatusResponse { remaining_alarms })
    }

    pub(super) fn try_query_prices(
        &self,
        block_time: Timestamp,
    ) -> Result<Vec<SpotPrice>, ContractError> {
        self.calc_all_prices(block_time).try_fold(
            vec![],
            |mut v: Vec<SpotPrice>,
             price: Result<BasePrice<SwapGroup, OracleBase>, ContractError>| {
                price.map(|price| {
                    v.push(price.into());

                    v
                })
            },
        )
    }

    pub(super) fn try_query_price(
        &self,
        at: Timestamp,
        currency: &SymbolOwned,
    ) -> Result<SpotPrice, ContractError> {
        self.feeds
            .calc_price(self.storage.deref(), &self.tree, currency, at, self.feeders)
    }

    fn calc_all_prices(
        &self,
        at: Timestamp,
    ) -> impl Iterator<Item = CalculateAllPricesIterItem<OracleBase>> + '_ {
        self.feeds.all_prices_iter(
            self.storage.deref(),
            self.tree.swap_pairs_df(),
            at,
            self.feeders,
        )
    }
}

impl<'storage, S, OracleBase> Oracle<'storage, S, OracleBase>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
    OracleBase: Currency + DeserializeOwned,
{
    const REPLY_ID: Id = 0;
    const EVENT_TYPE: &'_ str = "pricealarm";

    pub(super) fn try_notify_alarms(
        &mut self,
        block_time: Timestamp,
        max_count: u32,
    ) -> ContractResult<(u32, MessageResponse)> {
        let subscribers: Vec<Addr> = MarketAlarms::new(self.storage.deref())
            .ensure_no_in_delivery()?
            .notify_alarms_iter::<_, OracleBase>(self.calc_all_prices(block_time))?
            .take(max_count.try_into()?)
            .collect::<ContractResult<Vec<Addr>>>()?;

        #[cfg(debug_assertions)]
        Self::assert_unique_subscribers(&subscribers);

        let mut alarms: MarketAlarms<'_, &mut (dyn Storage + 'storage)> =
            MarketAlarms::new(self.storage.deref_mut());

        subscribers
            .into_iter()
            .try_fold(
                AlarmsDispatcher::new(ExecuteAlarmMsg::PriceAlarm(), Self::EVENT_TYPE),
                move |mut dispatcher: AlarmsDispatcher<ExecuteAlarmMsg>, subscriber: Addr| {
                    dispatcher = dispatcher.send_to(&subscriber, Self::REPLY_ID)?;

                    alarms.out_for_delivery(subscriber).map(|()| dispatcher)
                },
            )
            .map(|dispatcher| (dispatcher.nb_sent(), dispatcher.into()))
    }

    #[cfg(debug_assertions)]
    fn assert_unique_subscribers(subscribers: &[Addr]) {
        use std::collections::HashSet;

        let set: HashSet<&Addr> = HashSet::from_iter(subscribers);

        assert_eq!(set.len(), subscribers.len());
    }
}

#[cfg(test)]
mod tests {
    use currency::{lpn::Usdc, native::Nls, Currency as _};
    use finance::{
        coin::{Amount, Coin},
        duration::Duration,
        percent::Percent,
        price,
    };
    use marketprice::{config::Config as PriceConfig, SpotPrice};
    use sdk::cosmwasm_std::{
        testing::{MockApi, MockQuerier, MockStorage},
        Addr, DepsMut, Empty, QuerierWrapper, Storage, Timestamp,
    };

    use crate::{
        alarms::Alarm,
        contract::alarms::MarketAlarms,
        state::{config::Config, supported_pairs::SupportedPairs},
        swap_tree,
    };

    use super::{feed::Feeds, feeder::Feeders, Oracle};

    type BaseCurrency = Usdc;

    type NlsCoin = Coin<Nls>;
    type UsdcCoin = Coin<Usdc>;

    #[test]
    fn test() {
        const SAMPLE_PERIOD_SECS: u32 = 3;
        const _: () = if SAMPLE_PERIOD_SECS < 3 {
            panic!("Bug is reproduced with minimum of 3 seconds for the sample period.");
        };

        let mut storage: MockStorage = MockStorage::new();

        let price_config: PriceConfig = PriceConfig::new(
            Percent::HUNDRED,
            Duration::from_secs(SAMPLE_PERIOD_SECS),
            1,
            Percent::HUNDRED,
        );

        let mut now_seconds: u64 = 1;

        init(&mut storage, &price_config);

        add_higher_alarm(&mut storage);

        feed_below_price(&price_config, &mut storage, now_seconds);

        now_seconds += 1;

        dispatch_1(&mut storage, now_seconds);

        add_lower_alarm(&mut storage);

        now_seconds += 1;

        for _ in 0..SAMPLE_PERIOD_SECS.checked_add(1).unwrap() {
            dispatch_0(&mut storage, now_seconds);

            now_seconds += 1;
        }

        feed_normal_price(&price_config, &mut storage, now_seconds);

        now_seconds += 1;

        dispatch_1(&mut storage, now_seconds);

        now_seconds += 1;

        // Bug happens on this step.
        dispatch_0(&mut storage, now_seconds);

        feed_below_price(&price_config, &mut storage, now_seconds);

        now_seconds += 1;

        dispatch_0(&mut storage, now_seconds);
    }

    #[track_caller]
    fn init(storage: &mut dyn Storage, price_config: &PriceConfig) {
        Feeders::try_register(
            DepsMut {
                storage,
                api: &MockApi::default(),
                querier: QuerierWrapper::new(&MockQuerier::<Empty>::new(&[])),
            },
            String::from("feeder"),
        )
        .unwrap();

        Config::new(String::from(BaseCurrency::TICKER), price_config.clone())
            .store(storage)
            .unwrap();

        SupportedPairs::<BaseCurrency>::new(swap_tree!(Usdc::TICKER, (1, Nls::TICKER)).into_tree())
            .unwrap()
            .save(storage)
            .unwrap();
    }

    #[track_caller]
    fn add_alarm(storage: &mut dyn Storage, below_nls_to_2: Amount, above_1_to_usdc: Amount) {
        let storage: &mut dyn Storage = storage;

        let mut alarms: MarketAlarms<'_, &mut dyn Storage> = MarketAlarms::new(storage);

        alarms
            .try_add_price_alarm::<BaseCurrency>(
                Addr::unchecked("1"),
                Alarm::new(
                    SpotPrice::new(NlsCoin::new(below_nls_to_2).into(), UsdcCoin::new(2).into()),
                    Some(SpotPrice::new(
                        NlsCoin::new(1).into(),
                        UsdcCoin::new(above_1_to_usdc).into(),
                    )),
                ),
            )
            .unwrap();
    }

    #[track_caller]
    fn add_higher_alarm(storage: &mut dyn Storage) {
        add_alarm(storage, 1, 3)
    }

    #[track_caller]
    fn add_lower_alarm(storage: &mut dyn Storage) {
        add_alarm(storage, 2, 2)
    }

    #[track_caller]
    fn feed_price(
        price_config: &PriceConfig,
        storage: &mut dyn Storage,
        now_seconds: u64,
        nls: Amount,
        usdc: Amount,
    ) {
        Feeds::<BaseCurrency>::with(price_config.clone())
            .feed_prices(
                storage,
                Timestamp::from_seconds(now_seconds),
                &Addr::unchecked("feeder"),
                &[price::total_of(NlsCoin::new(nls))
                    .is(UsdcCoin::new(usdc))
                    .into()],
            )
            .unwrap();
    }

    #[track_caller]
    fn feed_below_price(price_config: &PriceConfig, storage: &mut dyn Storage, now_seconds: u64) {
        feed_price(price_config, storage, now_seconds, 1, 1)
    }

    #[track_caller]
    fn feed_normal_price(price_config: &PriceConfig, storage: &mut dyn Storage, now_seconds: u64) {
        feed_price(price_config, storage, now_seconds, 1, 2)
    }

    #[track_caller]
    fn dispatch(storage: &mut dyn Storage, now_seconds: u64, expected_count: u32) {
        let mut oracle: Oracle<'_, &mut dyn Storage, _> =
            Oracle::<'_, _, BaseCurrency>::load(storage).unwrap();

        let alarms: u32 = oracle
            .try_notify_alarms(Timestamp::from_seconds(now_seconds), 16)
            .unwrap()
            .0;

        assert_eq!(alarms, expected_count);
    }

    #[track_caller]
    fn deliver(storage: &mut dyn Storage, count: u32) {
        let storage: &mut dyn Storage = storage;

        let mut alarms: MarketAlarms<'_, &mut dyn Storage> = MarketAlarms::new(storage);

        for _ in 0..count {
            alarms.last_delivered().unwrap();
        }
    }

    #[track_caller]
    fn dispatch_and_deliver(storage: &mut dyn Storage, now_seconds: u64, expected_count: u32) {
        dispatch(storage, now_seconds, expected_count);

        deliver(storage, expected_count)
    }

    #[track_caller]
    fn dispatch_0(storage: &mut dyn Storage, now_seconds: u64) {
        dispatch_and_deliver(storage, now_seconds, 0)
    }

    #[track_caller]
    fn dispatch_1(storage: &mut dyn Storage, now_seconds: u64) {
        dispatch_and_deliver(storage, now_seconds, 1)
    }
}
