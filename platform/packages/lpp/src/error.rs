use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Lpp Platform] [Std] {0}")]
    Std(#[from] StdError),
}

pub type Result<T> = core::result::Result<T, Error>;
