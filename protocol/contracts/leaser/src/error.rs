use std::num::TryFromIntError;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Leaser] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Leaser] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),

    #[error("[Leaser] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Leaser] {0}")]
    Lpp(#[from] lpp::error::ContractError),

    #[error("[Leaser] {0}")]
    OraclePlatform(#[from] oracle_platform::error::Error),

    #[error("[Leaser] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Leaser] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Leaser] ParseError {err:?}")]
    ParseError { err: String },

    #[error("[Leaser] {0}")]
    Reserve(#[from] reserve::error::Error),

    #[error("[Leaser] Cannot open lease with zero downpayment")]
    ZeroDownpayment {},

    #[error("[Leaser] Unknown currency symbol: {symbol:?}")]
    UnknownCurrency { symbol: String },

    #[error("[Leaser] No Liquidity")]
    NoLiquidity {},

    #[error("[Leaser] Invalid continuation key, cause: {err} ")]
    InvalidContinuationKey { err: String },
}
