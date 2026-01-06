use std::any::type_name;

use thiserror::Error;

use currency::error::Error as CurrencyError;
use dex::Error as DexError;
use finance::error::Error as FinanceError;
use lpp::stub::{
    Error as LppStubError, lender::Error as LppLenderError, loan::Error as LppLoanError,
};
use oracle::{api::alarms::Error as OracleAlarmError, stub::Error as OracleError};
use oracle_platform::error::Error as OraclePlatformError;
use platform::error::Error as PlatformError;
use profit::stub::Error as ProfitError;
use reserve::stub::Error as ReserveError;
use sdk::cosmwasm_std::StdError;
use timealarms::stub::Error as TimeAlarmsError;

pub use crate::position::PositionError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Lease] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Lease] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Lease] {0}")]
    CurrencyError(#[from] CurrencyError),

    #[error("[Lease] {0}")]
    FinanceError(#[from] FinanceError),

    #[error("[Lease] {0}")]
    PlatformError(#[from] PlatformError),

    #[error("[Lease] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Lease] {0}")]
    LppStubError(LppStubError),

    #[error("[Lease] {0}")]
    LppLoanError(#[from] LppLoanError),

    #[error("[Lease] {0}")]
    OpenLoanReq(LppLenderError),

    #[error("[Lease] {0}")]
    OpenLoanResp(LppLenderError),

    #[error("[Lease] {0}")]
    LppLenderError(#[from] LppLenderError),

    #[error("[Lease] {0}")]
    TimeAlarmsError(#[from] TimeAlarmsError),

    #[error("[Lease] {0}")]
    OracleError(#[from] OracleError),

    #[error("[Lease] {0}")]
    OracleAlarmError(#[from] OracleAlarmError),

    #[error("[Lease] {0}")]
    FetchOraclePrice(OraclePlatformError),

    #[error("[Lease] {0}")]
    CrateOracleRef(OraclePlatformError),

    #[error("[Lease] {0}")]
    ProfitError(#[from] ProfitError),

    #[error("[Lease] {0}")]
    DexError(#[from] DexError),

    #[error("[Lease] {0}")]
    ReserveError(#[from] ReserveError),

    #[error("[Lease] {0}")]
    PositionError(#[from] PositionError),

    #[error("[Lease] No payment sent")]
    NoPaymentError(),

    #[error("[Lease] The operation '{0}' is not supported in the current state")]
    UnsupportedOperation(String),

    #[error("[Lease] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),

    #[error("[Lease] Inconsistency not detected")]
    InconsistencyNotDetected(),

    #[error("[Lease] Failed to query Position Limits")]
    PositionLimitsQuery(StdError),

    #[error("[Lease] Failed to query Access Check")]
    CheckAccessQuery(StdError),
}

impl ContractError {
    pub fn unsupported_operation<Op>(op: Op) -> Self
    where
        Op: ToString,
    {
        Self::UnsupportedOperation(op.to_string())
    }

    pub fn broken_invariant_if<T>(check: bool, msg: &str) -> ContractResult<()> {
        if check {
            Err(Self::BrokenInvariant(type_name::<T>().into(), msg.into()))
        } else {
            Ok(())
        }
    }

    pub fn overflow(msg: &'static str) -> Self {
        ContractError::FinanceError(FinanceError::Overflow(msg))
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
