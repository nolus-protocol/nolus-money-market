use crate::alarms::Alarm;
use finance::currency::Currency;
use marketprice::{alarms::PriceAlarms, SpotPrice};
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Addr, Storage},
};

use crate::{
    msg::{AlarmsStatusResponse, DispatchAlarmsResponse},
    ContractError,
};

pub struct MarketAlarms;

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

    pub fn try_add_price_alarm<BaseC>(
        storage: &mut dyn Storage,
        addr: Addr,
        alarm: Alarm,
    ) -> Result<Response, ContractError>
    where
        BaseC: Currency,
    {
        if let Some(above) = alarm.above() {
            Self::PRICE_ALARMS.add_alarm_above::<BaseC>(storage, &addr, above)?;
        }
        Self::PRICE_ALARMS.add_alarm_below::<BaseC>(storage, &addr, alarm.below())?;
        Ok(Response::new())
    }

    pub fn try_notify_alarms<BaseC>(
        storage: &mut dyn Storage,
        mut batch: Batch,
        prices: &[SpotPrice],
        max_count: u32,
    ) -> Result<Response, ContractError>
    where
        BaseC: Currency,
    {
        let sent = Self::PRICE_ALARMS.notify::<BaseC>(storage, &mut batch, prices, max_count)?;
        Ok(Response::from(batch).set_data(to_binary(&DispatchAlarmsResponse(sent))?))
    }

    pub fn try_query_alarms<BaseC>(
        storage: &dyn Storage,
        prices: &[SpotPrice],
    ) -> Result<AlarmsStatusResponse, ContractError>
    where
        BaseC: Currency,
    {
        let remaining_alarms = Self::PRICE_ALARMS.query_alarms::<BaseC>(storage, prices)?;
        Ok(AlarmsStatusResponse { remaining_alarms })
    }
}
