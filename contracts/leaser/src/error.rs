use std::any::type_name;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Leaser] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Leaser] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Leaser] {0}")]
    Lpp(#[from] lpp::error::ContractError),

    #[error("[Leaser] {0}")]
    Oracle(#[from] oracle::error::ContractError),

    #[error("[Leaser] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Leaser] Unauthorized")]
    Unauthorized {},

    #[error(
        "[Leaser] LeaseHealthyLiability% must be less than LeaseMaxLiability% and LeaseInitialLiability% must be less or equal to LeaseHealthyLiability%"
    )]
    IvalidLiability {},

    #[error("[Leaser] ParseError {err:?}")]
    ParseError { err: String },

    #[error("[Leaser] Validation {0}")]
    Validation(String),

    #[error("[Leaser] Custom Error val: {val:?}")]
    CustomError { val: String },

    #[error("[Leaser] Cannot open lease with zero downpayment")]
    ZeroDownpayment {},

    #[error("[Leaser] Unknown currency symbol: {symbol:?}")]
    UnknownCurrency { symbol: String },

    #[error("[Leaser] No Liquidity")]
    NoLiquidity {},

    #[error("[Leaser] No DEX connectivity setup")]
    NoDEXConnectivitySetup {},

    #[error("[Leaser] DEX connectivity already setup")]
    DEXConnectivityAlreadySetup {},
}

impl ContractError {
    pub fn validation_err<T>(str: String) -> Self {
        Self::Validation(format!("[ {} ] {}", String::from(type_name::<T>()), str))
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
