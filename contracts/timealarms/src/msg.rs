use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAlarm { time: Timestamp },
    Notify(),
    /// Returns [`AlarmsDispatchResponse`] as response data.
    DispatchAlarms { max_amount: u32 },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    DispatchToAlarms {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    TimeAlarm(Timestamp),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlarmsDispatchResponse {
    NextAlarm {
        /// Timestamp in nanoseconds since the start of the Unix epoch
        unix_time: u64,
    },
    RemainingForDispatch {
        /// `min(remaining_alarms, u32::MAX) as u32`
        remaining_alarms: u32,
    },
}

