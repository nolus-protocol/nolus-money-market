use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};
use timealarms::stub::TimeAlarmsRef;

use crate::{CadenceHours, result::ContractResult};

const CONFIG: Item<Config> = Item::new("contract_state");

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "testing", derive(Debug))]
pub(crate) struct Config {
    cadence_hours: CadenceHours,
    settlement: Addr,
    time_alarms: TimeAlarmsRef,
}

impl Config {
    pub fn new(cadence_hours: CadenceHours, settlement: Addr, time_alarms: TimeAlarmsRef) -> Self {
        Self {
            cadence_hours,
            settlement,
            time_alarms,
        }
    }

    pub fn update_cadence_hours(self, cadence_hours: CadenceHours) -> Self {
        Self {
            cadence_hours,
            ..self
        }
    }

    pub const fn cadence_hours(&self) -> CadenceHours {
        self.cadence_hours
    }

    pub const fn settlement(&self) -> &Addr {
        &self.settlement
    }

    pub const fn time_alarms(&self) -> &TimeAlarmsRef {
        &self.time_alarms
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        CONFIG.load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        CONFIG.save(storage, self).map_err(Into::into)
    }
}
