use serde::de::DeserializeOwned;

use currency::{Currency, Group, SymbolOwned};
use finance::price::{base::BasePrice, dto::PriceDTO};
use platform::{
    dispatcher::{AlarmsDispatcher, Id},
    message::Response as MessageResponse,
};
use sdk::{
    cosmwasm_ext::as_dyn::storage,
    cosmwasm_std::{Addr, Timestamp},
};

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

pub(crate) type PriceResult<PriceG, OracleBase> =
    Result<BasePrice<PriceG, OracleBase>, ContractError>;

pub(crate) struct Oracle<S, PriceG, BaseC, BaseG>
where
    S: storage::Dyn,
    PriceG: Group,
    BaseC: Currency + DeserializeOwned,
    BaseG: Group,
{
    storage: S,
    tree: SupportedPairs<BaseC>,
    feeders: usize,
    feeds: Feeds<PriceG, BaseC, BaseG>,
}

impl<S, PriceG, BaseC, BaseG> Oracle<S, PriceG, BaseC, BaseG>
where
    S: storage::Dyn,
    PriceG: Group + Clone,
    BaseC: Currency + DeserializeOwned,
    BaseG: Group,
{
    pub fn load(storage: S) -> Result<Self, ContractError> {
        let tree = SupportedPairs::load(&storage)?;
        let feeders = Feeders::total_registered(&storage).map_err(ContractError::LoadFeeders)?;
        let config = Config::load(&storage).map_err(ContractError::LoadConfig)?;
        let feeds = Feeds::<PriceG, BaseC, BaseG>::with(config.price_config);

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
        MarketAlarms::new(&self.storage)
            .try_query_alarms::<_, BaseC>(self.calc_all_prices(block_time))
            .map(|remaining_alarms| AlarmsStatusResponse { remaining_alarms })
    }

    pub(super) fn try_query_prices(
        &self,
        block_time: Timestamp,
    ) -> Result<Vec<PriceDTO<PriceG, BaseG>>, ContractError> {
        self.calc_all_prices(block_time).try_fold(
            vec![],
            |mut v: Vec<PriceDTO<PriceG, BaseG>>,
             price: Result<BasePrice<PriceG, BaseC>, ContractError>| {
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
    ) -> Result<PriceDTO<PriceG, BaseG>, ContractError> {
        self.feeds
            .calc_price(&self.storage, &self.tree, currency, at, self.feeders)
    }

    fn calc_all_prices(
        &self,
        at: Timestamp,
    ) -> impl Iterator<Item = PriceResult<PriceG, BaseC>> + '_ {
        self.feeds
            .all_prices_iter(&self.storage, self.tree.swap_pairs_df(), at, self.feeders)
    }
}

impl<S, PriceG, BaseC, BaseG> Oracle<S, PriceG, BaseC, BaseG>
where
    S: storage::DynMut,
    PriceG: Group + Clone,
    BaseC: Currency + DeserializeOwned,
    BaseG: Group,
{
    const REPLY_ID: Id = 0;
    const EVENT_TYPE: &'static str = "pricealarm";

    pub(super) fn try_notify_alarms(
        &mut self,
        block_time: Timestamp,
        max_count: u32,
    ) -> ContractResult<(u32, MessageResponse)> {
        let subscribers: Vec<Addr> = MarketAlarms::new(&self.storage)
            .ensure_no_in_delivery()?
            .notify_alarms_iter::<_, BaseC>(self.calc_all_prices(block_time))?
            .take(max_count.try_into()?)
            .collect::<ContractResult<Vec<Addr>>>()?;

        #[cfg(debug_assertions)]
        Self::assert_unique_subscribers(&subscribers);

        let mut alarms: MarketAlarms<_, PriceG> = MarketAlarms::new(&mut self.storage);

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
        test::{NativeC, StableC1},
        Lpns,
    };
    use currency::Currency as _;
    use finance::{coin::Coin, duration::Duration, percent::Percent, price};
    use marketprice::config::Config as PriceConfig;
    use sdk::{
        cosmwasm_ext::as_dyn::AsDynMut,
        cosmwasm_std::{
            testing::{MockApi, MockQuerier, MockStorage},
            Addr, DepsMut, Empty, QuerierWrapper, Timestamp,
        },
    };

    use crate::{
        api::{Alarm, Config, PriceCurrencies},
        contract::alarms::MarketAlarms,
        state::supported_pairs::SupportedPairs,
        swap_tree,
    };

    use super::{feed::Feeds, feeder::Feeders, Oracle};

    type BaseCurrency = StableC1;
    type BaseGroup = Lpns;

    type NlsCoin = Coin<NativeC>;
    type UsdcCoin = Coin<StableC1>;

    const NOW: Timestamp = Timestamp::from_seconds(1);

    const PRICE_BASE: NlsCoin = Coin::new(1);
    const PRICE_QUOTE: UsdcCoin = Coin::new(1);

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

    #[track_caller]
    fn init<S>(storage: &mut S, price_config: &PriceConfig)
    where
        S: storage::DynMut + ?Sized,
    {
        Feeders::try_register(
            DepsMut {
                storage: storage.as_dyn_mut(),
                api: &MockApi::default(),
                querier: QuerierWrapper::new(&MockQuerier::<Empty>::new(&[])),
            },
            String::from("feeder"),
        )
        .unwrap();

        Config::new(String::from(BaseCurrency::TICKER), price_config.clone())
            .store(storage)
            .unwrap();

        SupportedPairs::<BaseCurrency>::new(
            swap_tree!({ base: StableC1::TICKER }, (1, NativeC::TICKER)).into_tree(),
        )
        .unwrap()
        .save(storage)
        .unwrap();
    }

    #[track_caller]
    fn add_alarm<S>(storage: &mut S)
    where
        S: storage::DynMut + ?Sized,
    {
        let mut alarms: MarketAlarms<_, PriceCurrencies> = MarketAlarms::new(storage);

        alarms
            .try_add_price_alarm::<BaseCurrency, _>(
                Addr::unchecked("1"),
                Alarm::<_, BaseGroup>::new(
                    price::total_of(PRICE_BASE).is(PRICE_QUOTE),
                    Some(price::total_of(PRICE_BASE).is(PRICE_QUOTE)),
                ),
            )
            .unwrap();
    }

    #[track_caller]
    fn feed_price<S>(price_config: &PriceConfig, storage: &mut S)
    where
        S: storage::DynMut + ?Sized,
    {
        Feeds::<PriceCurrencies, BaseCurrency, BaseGroup>::with(price_config.clone())
            .feed_prices(
                storage,
                NOW,
                &Addr::unchecked("feeder"),
                &[price::total_of(PRICE_BASE).is(PRICE_QUOTE).into()],
            )
            .unwrap();
    }

    #[track_caller]
    fn dispatch<S>(storage: &mut S, expected_count: u32)
    where
        S: storage::DynMut + ?Sized,
    {
        let mut oracle: Oracle<_, PriceCurrencies, BaseCurrency, BaseGroup> =
            Oracle::load(storage).unwrap();

        let alarms: u32 = oracle.try_notify_alarms(NOW, 16).unwrap().0;

        assert_eq!(alarms, expected_count);
    }

    #[track_caller]
    fn deliver<S>(storage: &mut S, count: u32)
    where
        S: storage::DynMut + ?Sized,
    {
        let mut alarms: MarketAlarms<_, PriceCurrencies> = MarketAlarms::new(storage);

        for _ in 0..count {
            alarms.last_delivered().unwrap();
        }
    }

    #[track_caller]
    fn dispatch_and_deliver<S>(storage: &mut S, expected_count: u32)
    where
        S: storage::DynMut + ?Sized,
    {
        dispatch(storage, expected_count);

        deliver(storage, expected_count)
    }
}
