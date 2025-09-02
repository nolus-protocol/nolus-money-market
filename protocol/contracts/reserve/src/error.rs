use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Reserve] [Std] {0}")]
    Std(String),

    #[error("[Reserve] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Reserve] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Reserve] {0}")]
    ObtainBalance(platform::error::Error),

    #[error("[Reserve] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Reserve] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Reserve] Insufficient balance")]
    InsufficientBalance,
}

pub type Result<T> = std::result::Result<T, Error>;
