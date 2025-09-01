use thiserror::Error;

use platform::error::Error as PlatformError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp Platform] [Std] {0}")]
    Std(String),

    #[error("[Lpp Platform] [Platform] {0}")]
    Platform(#[from] PlatformError),
}

pub type Result<T> = core::result::Result<T, Error>;
