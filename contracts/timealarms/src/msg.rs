use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};
use time_oracle::AlarmsCount;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAlarm {
        time: Timestamp,
    },
    /// Returns [`DispatchAlarmsResponse`] as response data.
    DispatchAlarms {
        max_count: AlarmsCount,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ContractVersion {},
    AlarmsStatus {},
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    TimeAlarm {},
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
/// number of sent alarms
pub struct DispatchAlarmsResponse(pub AlarmsCount);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AlarmsStatusResponse {
    pub remaining_alarms: bool,
}
