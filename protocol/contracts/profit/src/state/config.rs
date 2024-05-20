use serde::{Deserialize, Serialize};

use currencies::Lpn as QuoteC;
use oracle::stub::SwapPath;
use oracle_platform::OracleRef;
use sdk::cosmwasm_std::Addr;
use timealarms::stub::TimeAlarmsRef;

use crate::typedefs::CadenceHours;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct Config {
    cadence_hours: CadenceHours,
    treasury: Addr,
    oracle: OracleRef<QuoteC>,
    time_alarms: TimeAlarmsRef,
}

impl Config {
    pub fn new(
        cadence_hours: CadenceHours,
        treasury: Addr,
        oracle: OracleRef<QuoteC>,
        time_alarms: TimeAlarmsRef,
    ) -> Self {
        Self {
            cadence_hours,
            treasury,
            oracle,
            time_alarms,
        }
    }

    pub fn update(self, cadence_hours: CadenceHours) -> Self {
        Self {
            cadence_hours,
            ..self
        }
    }

    pub fn cadence_hours(&self) -> CadenceHours {
        self.cadence_hours
    }

    pub fn treasury(&self) -> &Addr {
        &self.treasury
    }

    pub fn oracle(&self) -> &impl SwapPath {
        &self.oracle
    }

    pub fn time_alarms(&self) -> &TimeAlarmsRef {
        &self.time_alarms
    }
}
