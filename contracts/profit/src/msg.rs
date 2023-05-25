use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};

use crate::state::CadenceHours;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct InstantiateMsg {
    pub cadence_hours: CadenceHours,
    pub treasury: Addr,
    pub oracle: Addr,
    pub timealarms: Addr,
}

#[derive(Serialize, Deserialize)]
pub struct MigrateMsg {
    pub owner: Addr,
    pub oracle: Addr,
    pub timealarms: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    TimeAlarm {},
    Config { cadence_hours: CadenceHours },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct ConfigResponse {
    pub cadence_hours: CadenceHours,
}
