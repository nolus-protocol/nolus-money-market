use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Dex] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Swap(#[from] swap::error::Error),

    #[error("[Dex] The operation '{0}' is not supported in the current state")]
    UnsupportedOperation(String),

    #[error("[Dex] {0}")]
    OracleError(#[from] oracle::error::ContractError),

    #[error("[Dex] {0}")]
    TimeAlarmError(#[from] timealarms::error::ContractError),
    // #[error("[Swap] Expected response to {0} is not found")]
    // MissingResponse(String),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    pub fn unsupported_operation<Op>(op: Op) -> Self
    where
        Op: Into<String>,
    {
        Self::UnsupportedOperation(op.into())
    }
}
