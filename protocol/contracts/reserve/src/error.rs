use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Reserve] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Reserve] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Reserve] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Reserve] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Reserve] Insufficient balance")]
    InsufficientBalance,

    #[error("[Reserve][Stub] No response received from the Reserve contract")]
    NoResponseStub,
}

pub type Result<T> = std::result::Result<T, Error>;