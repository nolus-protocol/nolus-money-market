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
    AddAlarm {
        time: Timestamp,
    },
    /// Returns [`DispatchAlarmsResponse`] as response data.
    DispatchAlarms {
        max_count: u32,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AlarmsStatus {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    TimeAlarm(Timestamp),
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// number of sent alarms
pub struct DispatchAlarmsResponse(pub u32);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AlarmsStatusResponse {
    pub remaining_alarms: bool,
}
