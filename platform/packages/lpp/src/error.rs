use thiserror::Error;

use platform::error::Error as PlatformError;
use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp Platform] [Std] {0}")]
    Std(String),

    #[error("[Lpp Platform] [Platform] {0}")]
    Platform(#[from] PlatformError),
}

impl From<StdError> for Error {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}

pub type Result<T> = core::result::Result<T, Error>;
