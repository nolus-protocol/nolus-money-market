/// TODO remove once Astroport bump their dependency to cosmwasm-std 2.x
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Addr, Decimal, Uint128};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Receive receives a message of type [`Cw20ReceiveMsg`] and processes it depending on the received template
    Receive(Cw20ReceiveMsg),
    /// ExecuteSwapOperations processes multiple swaps while mentioning the minimum amount of tokens to receive for the last swap operation
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        max_spread: Option<Decimal>,
    },

    /// Internal use
    /// ExecuteSwapOperation executes a single swap operation
    ExecuteSwapOperation {
        operation: SwapOperation,
        to: Option<String>,
        max_spread: Option<Decimal>,
        single: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(rename_all = "snake_case")]
pub enum SwapOperation {
    /// Native swap
    NativeSwap {
        /// The name (denomination) of the native asset to swap from
        offer_denom: String,
        /// The name (denomination) of the native asset to swap to
        ask_denom: String,
    },
    /// ASTRO swap
    AstroSwap {
        /// Information about the asset being swapped
        offer_asset_info: AssetInfo,
        /// Information about the asset we swap to
        ask_asset_info: AssetInfo,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(rename_all = "snake_case")]
pub struct SwapResponseData {
    pub return_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[allow(clippy::derive_partial_eq_without_eq)] // Allow users of `#[cw_serde]` to not implement Eq without clippy complaining
#[serde(rename_all = "snake_case")]
pub struct Cw20ReceiveMsg; // originally definied in [cw20][https://crates.io/crates/cw20], unused in this codebase
