use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[RemoteLease] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[RemoteLease] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[RemoteLease] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[RemoteLease] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[RemoteLease] {0} must be non-empty")]
    EmptyInstantiateField(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
