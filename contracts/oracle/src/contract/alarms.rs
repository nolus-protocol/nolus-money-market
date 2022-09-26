use std::collections::HashSet;

use cosmwasm_std::{Addr, Response, StdResult, Storage};
use finance::{currency::SymbolOwned, price::dto::PriceDTO};
use marketprice::alarms::{price::PriceHooks, Alarm};
use platform::batch::Batch;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct MarketAlarms {}

impl MarketAlarms {
    const PRICE_ALARMS: PriceHooks<'static> = PriceHooks::new("hooks", "hooks_sequence");

    pub fn remove(storage: &mut dyn Storage, addr: Addr) -> Result<Response, ContractError> {
        Ok(Self::PRICE_ALARMS.remove(storage, addr)?)
    }

    pub fn try_add_price_alarm(
        storage: &mut dyn Storage,
        addr: Addr,
        alarm: Alarm,
    ) -> Result<Response, ContractError> {
        Self::PRICE_ALARMS.add_or_update(storage, &addr, alarm)?;
        Ok(Response::new())
    }

    pub fn get_hooks_currencies(storage: &dyn Storage) -> StdResult<HashSet<SymbolOwned>> {
        Self::PRICE_ALARMS.get_hook_denoms(storage)
    }

    pub fn try_notify_hooks(
        storage: &mut dyn Storage,
        updated_prices: Vec<PriceDTO>,
        batch: &mut Batch,
    ) -> Result<(), ContractError> {
        Ok(Self::PRICE_ALARMS.notify(storage, updated_prices, batch)?)
    }
}
