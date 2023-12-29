use currencies::{LeaseGroup, Lpns, PaymentGroup};
use serde::{Deserialize, Serialize};

use currency::SymbolOwned;
use marketprice::{config::Config as PriceConfig, SpotPrice};
use sdk::schemars::{self, JsonSchema};
use swap::SwapTarget;
use tree::HumanReadableTree;

pub use super::alarms::Alarm;

pub type StableCurrency = Lpns;
pub type AlarmCurrencies = LeaseGroup;
pub type PriceCurrencies = PaymentGroup;
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
        alarm: Alarm<AlarmCurrencies, StableCurrency>,
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

/// Implementation of oracle_platform::msg::Config
#[derive(Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug, Clone))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Config {
    pub base_asset: SymbolOwned,
    pub price_config: PriceConfig,
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
