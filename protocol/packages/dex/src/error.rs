use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Dex] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Swap(#[from] swap::error::Error),

    #[error("[Dex] The operation '{0}' is not supported in the current state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Dex] {0}")]
    OracleSwapError(#[from] oracle::api::swap::Error),

    #[error("[Dex] {0}")]
    OraclePlatformError(#[from] oracle_platform::error::Error),

    #[error("[Dex] {0}")]
    TimeAlarmError(#[from] timealarms::error::ContractError),
}

#[cfg(any(feature = "astroport", feature = "osmosis"))]
pub type Result<T> = core::result::Result<T, Error>;

#[cfg(any(feature = "astroport", feature = "osmosis"))]
impl Error {
    pub(crate) fn unsupported_operation<Op, State>(op: Op, state: State) -> Self
    where
        Op: Into<String>,
        State: std::fmt::Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{state}"))
    }
}
