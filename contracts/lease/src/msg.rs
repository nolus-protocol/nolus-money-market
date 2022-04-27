use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    /// The customer who wants to open a lease.
    pub customer: String,
    /// Denomination of the currency this lease will be about.
    pub currency: String,
    /// The delta, represented as permille, added on top of the LPP Loan interest rate.
    ///
    /// The value remains intact. The amount, a part of any payment, goes to the Profit contract.
    pub annual_margin_interest_permille: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ////////////////////
    /// Overseer operations
    ////////////////////

    /// Update config
    UpdateConfig {
        owner: Option<String>,
        liquidation_contract: Option<String>,
    },
    /// Make specified amount of tokens unspendable
    // LockCollateral { borrower: String, amount: Uint256 },
    /// Make specified amount of collateral tokens spendable
    // UnlockCollateral { borrower: String, amount: Uint256 },
    /// Claim bAsset rewards and distribute claimed rewards
    /// to market and overseer contracts
    DistributeRewards {},

    /// Liquidate collateral and send liquidated collateral to `to` address
    LiquidateCollateral {
        liquidator: String,
        borrower: String,
        // amount: Uint256,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Deposit collateral token
    DepositCollateral {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub collateral_token: String,
    pub overseer_contract: String,
    pub market_contract: String,
    pub reward_contract: String,
    pub liquidation_contract: String,
    pub stable_denom: String,
    pub basset_info: BAssetInfo,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowerResponse {
    pub borrower: String,
    // pub balance: Uint256,
    // pub spendable: Uint256,
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BorrowersResponse {
    pub borrowers: Vec<BorrowerResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BAssetInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}
