use std::num::TryFromIntError;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Leaser] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Leaser] Failed to serialize to JSON, cause: {0}")]
    SerializeToJson(StdError),

    #[error("[Leaser] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),

    #[error("[Leaser] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Leaser] {0}")]
    Lpp(#[from] lpp::error::Error),

    #[error("[Leaser] {0}")]
    PriceOracle(#[from] oracle_platform::error::Error),

    #[error("[Leaser] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Leaser] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Leaser] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Leaser] ParseError {err:?}")]
    ParseError { err: String },

    #[error("[Leaser] {0}")]
    Reserve(#[from] reserve::stub::Error),

    #[error("[Leaser] Cannot open lease with zero downpayment")]
    ZeroDownpayment {},

    #[error("[Leaser] No Liquidity")]
    NoLiquidity {},

    #[error("[Leaser] Invalid continuation key, cause: {err} ")]
    InvalidContinuationKey { err: String },

    #[error("[Leaser] The protocol is still in use. There are open leases")]
    ProtocolStillInUse(),

    #[error(
        "[Leaser][ProtocolsRegistry] The protocol deregistration request preparation failed! Cause: {0}"
    )]
    ProtocolDeregistration(platform::error::Error),

    #[error("[Leaser] Failed to query for the Lease package, cause: {0}")]
    QueryLeasePackage(StdError),
}
