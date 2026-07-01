use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[DrainVault] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[DrainVault] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[DrainVault] {0}")]
    ObtainBalance(platform::error::Error),

    #[error("[DrainVault] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[DrainVault] {0}")]
    Unauthorized(#[from] access_control::error::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
