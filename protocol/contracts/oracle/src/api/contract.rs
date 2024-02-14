use serde::{Deserialize, Deserializer, Serialize, Serializer};

use currencies::{Lpns, PaymentGroup};
use currency::SymbolOwned;
use finance::price::dto::PriceDTO;
use marketprice::config::Config as PriceConfig;
use sdk::{
    cosmwasm_std::Addr,
    schemars::{self, JsonSchema},
};
use tree::HumanReadableTree;

pub use super::alarms::Alarm;
use super::{alarms::AlarmCurrencies, swap::SwapTarget};

pub type BaseCurrencyGroup = Lpns;
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
        prices: Vec<PriceDTO<PriceCurrencies, PriceCurrencies>>,
    },
    AddPriceAlarm {
        alarm: Alarm<AlarmCurrencies, BaseCurrencyGroup>,
    },
    /// Returns [`DispatchAlarmsResponse`] as response data.
    DispatchAlarms { max_count: AlarmsCount },
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
    /// Provides a path in the swap tree between two arbitrary currencies
    ///
    /// Returns `oracle::api::swap::SwapPath`
    /// Implementation of oracle::api::swap::QueryMsg::SwapPath
    SwapPath {
        from: SymbolOwned,
        to: SymbolOwned,
    },
    /// Returns [`Status`] as response data.
    AlarmsStatus {},
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

pub type SupportedCurrencyPairsResponse = Vec<SwapLeg>;

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
    pub prices: Vec<PriceDTO<PriceCurrencies, BaseCurrencyGroup>>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
#[cfg_attr(any(test, feature = "testing"), derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AlarmsStatusResponse {
    pub remaining_alarms: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SwapLeg {
    pub from: SymbolOwned,
    pub to: SwapTarget,
}

impl Serialize for SwapLeg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.from, &self.to).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SwapLeg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|(from, to)| Self { from, to })
    }
}
