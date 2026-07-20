use thiserror::Error;

use crate::transport::SwapError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Swap(#[from] SwapError),

    #[error("[Dex] The operation '{0}' is not supported in the current state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Dex] {0}")]
    OracleSwap(#[from] oracle::api::swap::Error),

    #[error("[Dex] {0}")]
    MinOutput(oracle::stub::Error),

    #[error("[Dex] {0}")]
    TimeAlarm(#[from] timealarms::stub::Error),

    #[error("[Dex] {0}")]
    Unauthorized(access_control::error::Error),
}

pub type Result<T> = core::result::Result<T, Error>;

#[cfg(feature = "impl")]
impl Error {
    pub(crate) fn unsupported_operation<Op, State>(op: Op, state: State) -> Self
    where
        Op: Into<String>,
        State: std::fmt::Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{state}"))
    }
}
