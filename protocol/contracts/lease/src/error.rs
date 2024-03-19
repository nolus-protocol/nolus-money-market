use std::any::type_name;

use thiserror::Error;

use currency::error::Error as CurrencyError;
use dex::Error as DexError;
use finance::error::Error as FinanceError;
use lpp::error::ContractError as LppError;
use oracle::api::alarms::Error as OracleAlarmError;
use oracle_platform::error::Error as OraclePlatformError;
use platform::error::Error as PlatformError;
use profit::stub::Error as ProfitError;
use sdk::cosmwasm_std::StdError;
use timealarms::error::ContractError as TimeAlarmsError;

use crate::api::{LpnCoinDTO, PaymentCoin};

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

    #[error("[Lease] {0}")]
    LppError(#[from] LppError),

    #[error("[Lease] {0}")]
    TimeAlarmsError(#[from] TimeAlarmsError),

    #[error("[Lease] {0}")]
    OracleAlarmError(#[from] OracleAlarmError),

    #[error("[Lease] {0}")]
    OraclePlatformError(#[from] OraclePlatformError),

    #[error("[Lease] {0}")]
    ProfitError(#[from] ProfitError),

    #[error("[Lease] {0}")]
    DexError(#[from] DexError),

    #[error("[Lease] No payment sent")]
    NoPaymentError(),

    #[error("[Lease] Insufficient payment amount {0}")]
    InsufficientPayment(PaymentCoin),

    #[error("[Lease] Insufficient liquidation amount")]
    InsufficientLiquidation(),

    #[error("[Lease] The asset amount should worth at least {0}")]
    InsufficientAssetAmount(LpnCoinDTO),

    #[error("[Lease] The transaction amount should worth at least {0}")]
    InsufficientTransactionAmount(LpnCoinDTO),

    #[error("[Lease] The position close amount should worth at least {0}")]
    PositionCloseAmountTooSmall(LpnCoinDTO),

    #[error("[Lease] The position past this close should worth at least {0}")]
    PositionCloseAmountTooBig(LpnCoinDTO),

    #[error("[Lease] The operation '{0}' is not supported in the current state")]
    UnsupportedOperation(String),

    #[error("[Lease] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),

    #[error("[Lease] Inconsistency not detected")]
    InconsistencyNotDetected(),
}

impl ContractError {
    pub fn unsupported_operation<Op>(op: Op) -> Self
    where
        Op: Into<String>,
    {
        Self::UnsupportedOperation(op.into())
    }

    pub fn broken_invariant_if<T>(check: bool, msg: &str) -> ContractResult<()> {
        if check {
            Err(Self::BrokenInvariant(type_name::<T>().into(), msg.into()))
        } else {
            Ok(())
        }
    }
}

pub type ContractResult<T> = Result<T, ContractError>;
