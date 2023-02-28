use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("[Admin] [Std] {0}")]
    StdError(#[from] StdError),
    #[error("[Admin] {0}")]
    Platform(#[from] platform::error::Error),
    #[error("No data in migration response!")]
    NoMigrationResponseData {},
    #[error("Contract returned wrong release string! \"{reported}\" was returned, but \"{expected}\" was expected!")]
    WrongRelease { reported: String, expected: String },
}

#[derive(Debug, Error)]
#[error("This is unreachable!")]
pub enum Never {}

impl From<Never> for ContractError {
    fn from(value: Never) -> Self {
        match value {}
    }
}
