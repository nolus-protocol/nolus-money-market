use std::fmt::Display;

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
    OracleError(#[from] oracle::error::ContractError),

    #[error("[Dex] {0}")]
    TimeAlarmError(#[from] timealarms::error::ContractError),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    pub fn unsupported_operation<Op, State>(op: Op, state: State) -> Self
    where
        Op: Into<String>,
        State: Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{state}"))
    }
}
