use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Swap] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Swap] {0}")]
    Platform(#[from] platform::error::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
