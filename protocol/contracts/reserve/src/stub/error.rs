use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Reserve] [Stub] Failed to obtain contract's Lpn. Cause: {0}")]
    QueryReserveFailure(String),

    #[error("[Reserve][Stub] Contacted a reserve contract with unexpected Lpn. Cause: {0}")]
    UnexpectedLpn(currency::error::Error),

    #[error("[Reserve] {0}")]
    Platform(#[from] platform::error::Error),
}
