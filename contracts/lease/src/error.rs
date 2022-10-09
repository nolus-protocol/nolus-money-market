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
    #[error("[Lease] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lease] Unauthorized")]
    Unauthorized {},

    #[error("[Lease] {0}")]
    FinanceError(#[from] FinanceError),

    #[error("[Lease] {0}")]
    PlatformError(#[from] PlatformError),

    #[error("[Lease] {0}")]
    LppError(#[from] LppError),

    #[error("[Lease] {0}")]
    TimeAlarmsError(#[from] TimeAlarmsError),

    #[error("[Lease] {0}")]
    OracleError(#[from] OracleError),

    #[error("[Lease] {0}")]
    ProfitError(#[from] ProfitError),

    #[error("[Lease] No downpayment sent")]
    NoDownpaymentError(),

    #[error("[Lease] The underlying loan is not fully repaid")]
    LoanNotPaid(),

    #[error("[Lease] The underlying loan is closed")]
    LoanClosed(),

    #[error("[Lease] Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("[Lease] The operation '{0}' is not supported in state '{1}'")]
    UnsupportedOperation(String, String),
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
