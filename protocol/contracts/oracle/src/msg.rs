use serde::{Deserialize, Serialize};

use currency::SymbolOwned;
use marketprice::{config::Config as PriceConfig, SpotPrice};
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};
use swap::SwapTarget;
use tree::HumanReadableTree;

use crate::{
    alarms::Alarm,
    state::{config::Config, supported_pairs::SwapLeg},
};

pub type AlarmsCount = platform::dispatcher::AlarmsCount;

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub config: Config,
    pub swap_tree: HumanReadableTree<SwapTarget>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteMsg {
    FeedPrices {
        prices: Vec<SpotPrice>,
    },
    AddPriceAlarm {
        alarm: Alarm,
    },
    /// Returns [`DispatchAlarmsResponse`] as response data.
    DispatchAlarms {
        max_count: AlarmsCount,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum SudoMsg {
    RegisterFeeder { feeder_address: String },
    RemoveFeeder { feeder_address: String },
    UpdateConfig(PriceConfig),
    SwapTree { tree: HumanReadableTree<SwapTarget> },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum QueryMsg {
    // Returns contract's semantic version as a package, which is set in `Cargo.toml`.
    ContractVersion {},
    // returns the contract configuration
    Config {},
    // returns the supported currencies tree
    SwapTree {},
    // returns all registered feeders
    Feeders {},
    // check if an address belongs to a registered feeder
    IsFeeder {
        address: Addr,
    },
    /// Provides all supported prices
    Prices {},
    /// Provides the price of the currency against the base asset
    ///
    /// Implementation of oracle_platform::msg::QueryMsg::Price
    Price {
        currency: SymbolOwned,
    },
    // returns a list of supported denom pairs
    SupportedCurrencyPairs {},
    SwapPath {
        from: SymbolOwned,
        to: SymbolOwned,
    },
    /// Returns [`Status`] as response data.
    AlarmsStatus {},
}

pub type SupportedCurrencyPairsResponse = Vec<SwapLeg>;

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ConfigResponse {
    pub config: Config,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SwapTreeResponse {
    pub tree: HumanReadableTree<SwapTarget>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PricesResponse {
    pub prices: Vec<SpotPrice>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum ExecuteAlarmMsg {
    PriceAlarm(),
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "testing", derive(PartialEq, Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DispatchAlarmsResponse(pub AlarmsCount);

#[derive(Serialize, Deserialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AlarmsStatusResponse {
    pub remaining_alarms: bool,
}
