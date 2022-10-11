use serde::{Deserialize, Serialize};

use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    schemars::{self, JsonSchema},
};

use self::tvl_intervals::Intervals;

pub mod config;
pub mod dispatch_log;
pub mod tvl_intervals;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct DispatchLog {
    pub last_dispatch: Timestamp,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    // Time duration in hours defining the periods of time this instance is awaken
    pub cadence_hours: u16,
    // An LPP instance address
    pub lpp: Addr,
    // address to treasury contract
    pub treasury: Addr,
    // address to oracle contract
    pub oracle: Addr,
    // address to time alarms contract
    pub timealarms: Addr,
    // A list of (minTVL_MNLS: u32, APR%o) which defines the APR as per the TVL.
    pub tvl_to_apr: Intervals,
}
