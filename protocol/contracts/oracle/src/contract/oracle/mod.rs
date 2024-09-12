use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use currency::{Currency, CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::{
    base::{
        with_price::{self, WithPrice},
        BasePrice,
    },
    Price,
};
use platform::{
    dispatcher::{AlarmsDispatcher, Id},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Storage, Timestamp};

use crate::{
    api::{AlarmsStatusResponse, Config, ExecuteAlarmMsg},
    contract::{alarms::MarketAlarms, oracle::feed::Feeds},
    error::ContractError,
    result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

use self::feeder::Feeders;

pub mod feed;
pub mod feeder;

pub(crate) type PriceResult<PriceG, OracleBase, OracleBaseG> =
    Result<BasePrice<PriceG, OracleBase, OracleBaseG>, ContractError>;

pub(crate) struct Oracle<'storage, S, PriceG, BaseC, BaseG>
where
    S: Deref<Target = dyn Storage + 'storage>,
    PriceG: Group,
    BaseC: Currency + MemberOf<BaseG>,
    BaseG: Group,
{
    storage: S,
    tree: SupportedPairs<PriceG, BaseC>,
    feeders: usize,
    feeds: Feeds<PriceG, BaseC, BaseG>,
}

impl<'storage, S, PriceG, BaseC, BaseG> Oracle<'storage, S, PriceG, BaseC, BaseG>
where
    S: Deref<Target = dyn Storage + 'storage>,
    PriceG: Group<TopG = PriceG>,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
{
    pub fn load(storage: S) -> Result<Self, ContractError> {
        let tree = SupportedPairs::load(storage.deref())?;
        let feeders =
            Feeders::total_registered(storage.deref()).map_err(ContractError::LoadFeeders)?;
        Config::load(storage.deref())
            .map(|cfg| Feeds::<PriceG, BaseC, BaseG>::with(cfg.price_config))
            .map(|feeds| Self {
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
            .try_query_alarms::<_, BaseC, BaseG>(self.calc_all_prices(block_time))
            .map(|remaining_alarms| AlarmsStatusResponse { remaining_alarms })
    }

    pub(super) fn try_query_prices(
        &self,
        block_time: Timestamp,
    ) -> Result<Vec<BasePrice<PriceG, BaseC, BaseG>>, ContractError> {
        self.calc_all_prices(block_time).collect()
    }

    pub(super) fn try_query_base_price(
        &self,
        at: Timestamp,
        currency: &CurrencyDTO<PriceG>,
    ) -> Result<BasePrice<PriceG, BaseC, BaseG>, ContractError> {
        self.feeds
            .calc_base_price(self.storage.deref(), &self.tree, currency, at, self.feeders)
    }

    pub(super) fn try_query_stable_price<StableCurrency>(
        &self,
        at: Timestamp,
        currency: &CurrencyDTO<PriceG>,
    ) -> Result<BasePrice<PriceG, StableCurrency, PriceG>, ContractError>
    where
        StableCurrency: CurrencyDef,
        StableCurrency::Group: MemberOf<PriceG>,
    {
        struct StablePriceCalc<G, StableCurrency, StableG, BaseCurrency> {
            _currency_group: PhantomData<G>,
            stable_to_base: Price<StableCurrency, BaseCurrency>,
            _quote_group: PhantomData<StableG>,
        }
        impl<G, StableCurrency, StableG, BaseCurrency> WithPrice<BaseCurrency>
            for StablePriceCalc<G, StableCurrency, StableG, BaseCurrency>
        where
            StableCurrency: CurrencyDef,
            StableCurrency::Group: MemberOf<StableG>,
            BaseCurrency: CurrencyDef,
            G: Group,
            StableG: Group,
        {
            type PriceG = G;

            type Output = BasePrice<G, StableCurrency, StableG>;

            type Error = ContractError;

            fn exec<BaseC>(
                self,
                base_price: Price<BaseC, BaseCurrency>,
            ) -> Result<Self::Output, Self::Error>
            where
                BaseC: CurrencyDef,
                BaseC::Group: MemberOf<Self::PriceG>,
            {
                Ok((base_price * self.stable_to_base.inv()).into())
            }
        }
        self.try_query_base_price(at, &currency::dto::<StableCurrency, _>())
            .and_then(|stable_price| {
                Price::try_from(&stable_price).map_err(Into::<ContractError>::into)
            })
            .and_then(|stable_price: Price<StableCurrency, BaseC>| {
                self.try_query_base_price(at, currency)
                    .and_then(|ref base_price| {
                        with_price::execute(
                            base_price,
                            StablePriceCalc {
                                _currency_group: PhantomData::<PriceG>,
                                stable_to_base: stable_price,
                                _quote_group: PhantomData::<PriceG>,
                            },
                        )
                    })
            })
    }

    fn calc_all_prices(
        &self,
        at: Timestamp,
    ) -> impl Iterator<Item = PriceResult<PriceG, BaseC, BaseG>> + '_ {
        self.feeds.all_prices_iter(
            self.storage.deref(),
            self.tree.swap_pairs_df(),
            at,
            self.feeders,
        )
    }
}

impl<'storage, S, PriceG, BaseC, BaseG> Oracle<'storage, S, PriceG, BaseC, BaseG>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
    PriceG: Group<TopG = PriceG> + Clone,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<BaseG> + MemberOf<PriceG>,
    BaseG: Group + MemberOf<PriceG>,
{
    const REPLY_ID: Id = 0;
    const EVENT_TYPE: &'static str = "pricealarm";

    pub(super) fn try_notify_alarms(
        &mut self,
        block_time: Timestamp,
        max_count: u32,
    ) -> ContractResult<(u32, MessageResponse)> {
        let subscribers: Vec<Addr> = MarketAlarms::new(self.storage.deref())
            .ensure_no_in_delivery()?
            .notify_alarms_iter::<_, BaseC, BaseG>(self.calc_all_prices(block_time))?
            .take(max_count.try_into()?)
            .collect::<ContractResult<Vec<Addr>>>()?;

        #[cfg(debug_assertions)]
        Self::assert_unique_subscribers(&subscribers);

        let mut alarms: MarketAlarms<'_, &mut (dyn Storage + 'storage), PriceG> =
            MarketAlarms::new(self.storage.deref_mut());

        subscribers
            .into_iter()
            .try_fold(
                AlarmsDispatcher::new(ExecuteAlarmMsg::PriceAlarm(), Self::EVENT_TYPE),
                move |dispatcher: AlarmsDispatcher<ExecuteAlarmMsg>, subscriber: Addr| {
                    dispatcher
                        .send_to(subscriber.clone(), Self::REPLY_ID)
                        .map_err(Into::into)
                        .and_then(|dispatcher| {
                            alarms.out_for_delivery(subscriber).map(|()| dispatcher)
                        })
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
mod test_normalized_price_not_found {
    use currencies::{
        Lpn as BaseCurrency, Lpns as BaseCurrencies, Nls, PaymentC4,
        PaymentGroup as PriceCurrencies, PaymentGroup as AlarmCurrencies, Stable as StableCurrency,
    };
    use currency::CurrencyDef as _;
    use finance::{coin::Coin, duration::Duration, percent::Percent, price};
    use marketprice::config::Config as PriceConfig;
    use sdk::cosmwasm_std::{
        testing::{MockApi, MockQuerier, MockStorage},
        Addr, DepsMut, Empty, QuerierWrapper, Storage, Timestamp,
    };

    use crate::{
        api::{Alarm, Config},
        contract::alarms::MarketAlarms,
        state::supported_pairs::SupportedPairs,
        swap_tree,
    };

    use super::{feed::Feeds, feeder::Feeders, Oracle};

    type NlsCoin = Coin<Nls>;
    type BaseCoin = Coin<BaseCurrency>;

    const NOW: Timestamp = Timestamp::from_seconds(1);

    const PRICE_BASE: NlsCoin = Coin::new(1);
    const PRICE_QUOTE: BaseCoin = Coin::new(1);

    #[test]
    fn test() {
        let mut storage: MockStorage = MockStorage::new();

        let price_config: PriceConfig = PriceConfig::new(
            Percent::HUNDRED,
            Duration::from_secs(1),
            1,
            Percent::HUNDRED,
        );

        init(&mut storage, &price_config);

        add_alarm(&mut storage);

        feed_price(&price_config, &mut storage);

        dispatch_and_deliver(&mut storage, 1);

        // Bug happens on this step.
        dispatch_and_deliver(&mut storage, 0);
    }

    // #[track_caller]
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

        Config::new(price_config.clone()).store(storage).unwrap();

        SupportedPairs::<PriceCurrencies, BaseCurrency>::new::<StableCurrency>(
            swap_tree!({ base: BaseCurrency::ticker() }, (1, Nls::ticker()), (10, PaymentC4::ticker()))
                .into_tree(),
        )
        .unwrap()
        .save(storage)
        .unwrap();
    }

    #[track_caller]
    fn add_alarm(storage: &mut dyn Storage) {
        let mut alarms: MarketAlarms<'_, &mut dyn Storage, _> = MarketAlarms::new(storage);

        alarms
            .try_add_price_alarm::<BaseCurrency, BaseCurrencies>(
                Addr::unchecked("1"),
                Alarm::<AlarmCurrencies, BaseCurrency, BaseCurrencies>::new(
                    price::total_of(PRICE_BASE).is(PRICE_QUOTE),
                    Some(price::total_of(PRICE_BASE).is(PRICE_QUOTE)),
                ),
            )
            .unwrap();
    }

    #[track_caller]
    fn feed_price(price_config: &PriceConfig, storage: &mut dyn Storage) {
        Feeds::<PriceCurrencies, BaseCurrency, BaseCurrencies>::with(price_config.clone())
            .feed_prices(
                storage,
                NOW,
                &Addr::unchecked("feeder"),
                &[price::total_of(PRICE_BASE).is(PRICE_QUOTE).into()],
            )
            .unwrap();
    }

    #[track_caller]
    fn dispatch(storage: &mut dyn Storage, expected_count: u32) {
        let mut oracle: Oracle<
            '_,
            &mut dyn Storage,
            PriceCurrencies,
            BaseCurrency,
            BaseCurrencies,
        > = Oracle::load(storage).unwrap();

        let alarms: u32 = oracle.try_notify_alarms(NOW, 16).unwrap().0;

        assert_eq!(alarms, expected_count);
    }

    #[track_caller]
    fn deliver(storage: &mut dyn Storage, count: u32) {
        let mut alarms: MarketAlarms<'_, &mut dyn Storage, PriceCurrencies> =
            MarketAlarms::new(storage);

        for _ in 0..count {
            alarms.last_delivered().unwrap();
        }
    }

    #[track_caller]
    fn dispatch_and_deliver(storage: &mut dyn Storage, expected_count: u32) {
        dispatch(storage, expected_count);

        deliver(storage, expected_count)
    }
}
