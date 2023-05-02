use serde::de::DeserializeOwned;

use finance::currency::{Currency, SymbolOwned};
use marketprice::SpotPrice;
use platform::dispatcher::{AlarmsDispatcher, Id};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Storage, Timestamp};

use crate::{
    contract::{
        alarms::{MarketAlarms, PriceResult},
        oracle::feed::Feeds,
    },
    msg::{AlarmsStatusResponse, ExecuteAlarmMsg},
    result::ContractResult,
    state::{config::Config, supported_pairs::SupportedPairs},
    ContractError,
};

use self::feeder::Feeders;

pub mod feed;
pub mod feeder;

pub struct Oracle<OracleBase>
where
    OracleBase: Currency + DeserializeOwned,
{
    tree: SupportedPairs<OracleBase>,
    feeders: usize,
    feeds: Feeds<OracleBase>,
}

impl<OracleBase> Oracle<OracleBase>
where
    OracleBase: Currency + DeserializeOwned,
{
    const REPLY_ID: Id = 0;
    const EVENT_TYPE: &str = "pricealarm";

    pub fn load(storage: &dyn Storage) -> Result<Self, ContractError> {
        let tree = SupportedPairs::load(storage)?;
        let feeders = Feeders::total_registered(storage)?;
        let config = Config::load(storage)?;
        let feeds = Feeds::<OracleBase>::with(config.price_config);

        Ok(Self {
            tree,
            feeders,
            feeds,
        })
    }

    pub(super) fn try_notify_alarms(
        &self,
        storage: &dyn Storage,
        block_time: Timestamp,
        max_count: u32,
    ) -> ContractResult<(u32, MessageResponse)>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        let prices = self.calc_all_prices(storage, block_time)?;

        MarketAlarms::notify_alarms_iter::<OracleBase>(storage, prices, max_count.try_into()?)
            .try_fold(
                AlarmsDispatcher::new(ExecuteAlarmMsg::PriceAlarm(), Self::EVENT_TYPE),
                |dispatcher, receiver| {
                    dispatcher
                        .send_to(&receiver?, Self::REPLY_ID)
                        .map_err::<ContractError, _>(Into::into)
                },
            )
            .map(|dispatcher| (dispatcher.nb_sent(), dispatcher.into()))
    }

    pub(super) fn try_query_alarms(
        &self,
        storage: &dyn Storage,
        block_time: Timestamp,
    ) -> Result<AlarmsStatusResponse, ContractError>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        let prices = self.calc_all_prices(storage, block_time)?;
        let remaining_alarms = MarketAlarms::try_query_alarms::<OracleBase>(storage, prices)?;
        Ok(AlarmsStatusResponse { remaining_alarms })
    }

    pub(super) fn try_query_prices(
        &self,
        storage: &dyn Storage,
        block_time: Timestamp,
    ) -> Result<Vec<SpotPrice>, ContractError>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        self.calc_all_prices(storage, block_time)?
            .try_fold(vec![], |mut v, price| {
                v.push(price?.into());
                Ok(v)
            })
    }

    pub(super) fn try_query_price(
        &self,
        storage: &dyn Storage,
        at: Timestamp,
        currency: &SymbolOwned,
    ) -> Result<SpotPrice, ContractError>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        self.feeds
            .calc_price(storage, &self.tree, currency, at, self.feeders)
    }

    fn calc_all_prices<'a>(
        &'a self,
        storage: &'a dyn Storage,
        at: Timestamp,
    ) -> Result<impl Iterator<Item = PriceResult<OracleBase>> + 'a, ContractError>
    where
        OracleBase: Currency + DeserializeOwned,
    {
        Ok(self
            .feeds
            .all_prices_iter(storage, self.tree.swap_pairs_df(), at, self.feeders))
    }
}
