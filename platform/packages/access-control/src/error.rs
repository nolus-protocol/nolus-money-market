use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Access Control] Unauthorized access!")]
    Unauthorized {},

    #[error("[Access Control] [Std] {0}")]
    Std(String),
}

impl From<StdError> for Error {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}

pub type Result = std::result::Result<(), Error>;
