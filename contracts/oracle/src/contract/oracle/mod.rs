use std::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;

use finance::{
    currency::{Currency, SymbolOwned},
    price::base::BasePrice,
};
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
        let tree = SupportedPairs::load(&*storage)?;
        let feeders = Feeders::total_registered(&*storage)?;
        let config = Config::load(&*storage)?;
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
