use cosmwasm_std::StdError;
use finance::error::Error as FinanceError;
use platform::error::Error as PlatformError;
use lpp::error::ContractError as LppError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("{0}")]
    FinanceError(#[from] FinanceError),

    #[error("{0}")]
    PlatformError(#[from] PlatformError),

    #[error("{0}")]
    LppError(#[from] LppError),

    #[error("{symbol:?}")]
    UnknownCurrency { symbol: String },

    #[error("Error in opening an underlying loan: {0}")]
    OpenLoanError(String),

    #[error("The underlying loan is not fully repaid")]
    LoanNotPaid(),

    #[error("The underlying loan is closed")]
    LoanClosed(),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

pub type ContractResult<T> = core::result::Result<T, ContractError>;
