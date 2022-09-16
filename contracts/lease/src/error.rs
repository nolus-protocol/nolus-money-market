use std::fmt::Display;

use cosmwasm_std::StdError;
use thiserror::Error;

use finance::error::Error as FinanceError;
use lpp::error::ContractError as LppError;
use market_price_oracle::error::ContractError as OracleError;
use platform::error::Error as PlatformError;
use profit::error::ContractError as ProfitError;
use time_alarms::error::ContractError as TimeAlarmsError;

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

    #[error("{0}")]
    TimeAlarmsError(#[from] TimeAlarmsError),

    #[error("{0}")]
    OracleError(#[from] OracleError),

    #[error("{0}")]
    ProfitError(#[from] ProfitError),

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

    #[error("The operation '{0}' is not supported in state '{1}'")]
    UnsupportedOperation(String, String),
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.35/thiserror/ for details.
}

impl ContractError {
    pub fn unsupported_operation<Op, State>(op: Op, state: &State) -> Self
    where
        Op: Into<String>,
        State: Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{}", state))
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
