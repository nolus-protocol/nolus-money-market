use thiserror::Error;

use crate::swap;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Dex] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Swap(#[from] swap::Error),

    #[error("[Dex] The operation '{0}' is not supported in the current state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Dex] {0}")]
    OracleSwapError(#[from] oracle::api::swap::Error),

    #[error("[Dex] {0}")]
    MinOutput(oracle::stub::Error),

    #[error("[Dex] {0}")]
    TimeAlarmError(#[from] timealarms::stub::Error),

    #[error("[Dex] {0}")]
    Unauthorized(access_control::error::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    pub(crate) fn unsupported_operation<Op, State>(op: Op, state: State) -> Self
    where
        Op: Into<String>,
        State: std::fmt::Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{state}"))
    }
}
