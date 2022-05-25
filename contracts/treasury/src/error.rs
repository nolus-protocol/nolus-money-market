use cosmwasm_std::StdError;
use thiserror::Error;

use semver::Error as SemverError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Rewards dispatcher is not configured")]
    NotConfigured {},
}

impl From<SemverError> for ContractError {
    fn from(_: SemverError) -> Self {
        ContractError::Std(StdError::invalid_utf8("semver err"))
    }
}
