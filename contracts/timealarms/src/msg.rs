use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Timestamp,
    schemars::{self, JsonSchema},
};

pub type AlarmsCount = platform::dispatcher::AlarmsCount;

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddAlarm {
        time: Timestamp,
    },
    /// Returns [`DispatchAlarmsResponse`] as response data.
    DispatchAlarms {
        max_count: AlarmsCount,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    ContractVersion {},
    AlarmsStatus {},
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    TimeAlarm {},
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "testing", derive(PartialEq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
/// number of sent alarms
pub struct DispatchAlarmsResponse(pub AlarmsCount);

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AlarmsStatusResponse {
    pub remaining_alarms: bool,
}
