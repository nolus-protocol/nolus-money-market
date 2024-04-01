use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("[Dispatcher] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Dispatcher] {0}")]
    Platform(#[from] platform::error::Error),
}
