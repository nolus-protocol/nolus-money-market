use thiserror::Error;

use crate::transport::SwapError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Transport(SwapError),

    #[error("[Dex] The operation '{0}' is not supported in the current state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Dex] The remote lease operation response '{0}' is not a Swap response")]
    NotSwapResponse(String),

    #[error(
        "[Dex] The remote lease swap response amount '{0}' is not of the expected currency '{1}'. Details: {2}"
    )]
    IncorrectSwapOutCurrency(String, String, finance::error::Error),

    #[error("[Dex] {0}")]
    OracleSwap(#[from] oracle::api::swap::Error),

    #[error("[Dex] {0}")]
    MinOutput(oracle::stub::Error),

    #[error("[Dex] {0}")]
    TimeAlarm(#[from] timealarms::stub::Error),

    #[error("[Dex] {0}")]
    Unauthorized(access_control::error::Error),

    #[error("[Dex] Failed to build the swap request: {0}")]
    BuildSwapRequest(remote_lease::error::Error),

    #[error("[Dex] Arithmetic overflow: {0}")]
    Overflow(&'static str),
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
