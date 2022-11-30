use marketprice::alarms::{price::PriceAlarms, Alarm};
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage},
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};

use crate::ContractError;

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

    // TODO: separation of price feed and alarms notification
    /*
    pub fn try_notify_alarms(
        storage: &mut dyn Storage,
        updated_prices: Vec<SpotPrice>,
        batch: &mut Batch,
    ) -> Result<(), ContractError> {
        Ok(Self::PRICE_ALARMS.notify(storage, updated_prices, batch)?)
    }
    */
}
