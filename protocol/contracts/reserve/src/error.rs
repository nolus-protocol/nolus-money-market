use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Reserve] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Reserve] [Stub] Failed to obtain contract's Lpn. Cause: {0}")]
    QueryReserveFailure(StdError),

    #[error("[Reserve] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Reserve] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Reserve] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Reserve] Insufficient balance")]
    InsufficientBalance,

    #[error("[Reserve][Stub] Contacted a reserve contract with unexpected Lpn. Cause: {0}")]
    UnexpectedLpn(currency::error::Error),
}
