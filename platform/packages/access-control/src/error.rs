use sdk::cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Access Control] Unauthorized access!")]
    Unauthorized {},

    #[error("[Access Control] [Std] {0}")]
    Std(#[from] StdError),
}

pub type Result = std::result::Result<(), Error>;
