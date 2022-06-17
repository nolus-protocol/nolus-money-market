use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum HooksError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Error on add hook")]
    AddHook {},
}
