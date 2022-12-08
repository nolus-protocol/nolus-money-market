use cosmwasm_std::to_binary;
use marketprice::{alarms::{price::PriceAlarms, Alarm}, SpotPrice};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage},
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};

use crate::{ContractError, msg::{SentAlarmsResponse, AlarmsStatusResponse}};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MarketAlarms {}

impl MarketAlarms {
    const PRICE_ALARMS: PriceAlarms<'static> = PriceAlarms::new(
        "alarms_below",
        "index_below",
        "alarms_above",
        "index_above",
        "msg_id",
    );

    pub fn remove(storage: &mut dyn Storage, addr: Addr) -> Result<Response, ContractError> {
        Self::PRICE_ALARMS.remove(storage, addr)?;
        Ok(Response::default())
    }

    pub fn try_add_price_alarm(
        storage: &mut dyn Storage,
        addr: Addr,
        alarm: Alarm,
    ) -> Result<Response, ContractError> {
        Self::PRICE_ALARMS.add_or_update(storage, &addr, alarm)?;
        Ok(Response::new())
    }

    pub fn try_notify_alarms(
        storage: &mut dyn Storage,
        mut batch: Batch,
        prices: &[SpotPrice],
        max_count: u32,
    ) -> Result<Response, ContractError>
    {
        let sent = Self::PRICE_ALARMS.notify(storage, &mut batch, prices, max_count)?;
        Ok(Response::from(batch).set_data(to_binary(&SentAlarmsResponse(sent))?))
    }

    pub fn try_query_alarms(
        storage: &dyn Storage,
        prices: &[SpotPrice],
    ) -> Result<AlarmsStatusResponse, ContractError>
    {
        let remaining_alarms = Self::PRICE_ALARMS.query_alarms(storage, prices)?;
        Ok(AlarmsStatusResponse { remaining_alarms })
    }
}
