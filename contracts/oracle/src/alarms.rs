use std::collections::{HashMap, HashSet};

use cosmwasm_std::{Addr, Response, StdResult, Storage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use finance::currency::SymbolOwned;
use marketprice::{
    alarms::{price::PriceHooks, Alarm},
    storage::{Denom, Price},
};
use platform::batch::Batch;

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

    pub fn get_hooks_currencies(storage: &dyn Storage) -> StdResult<HashSet<Denom>> {
        Self::PRICE_ALARMS.get_hook_denoms(storage)
    }

    pub fn try_notify_hooks(
        storage: &mut dyn Storage,
        updated_prices: HashMap<SymbolOwned, Price>,
    ) -> Result<Batch, ContractError> {
        Ok(Self::PRICE_ALARMS.notify(storage, updated_prices)?)
    }
}
