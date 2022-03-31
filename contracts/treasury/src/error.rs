use cosmwasm_std::StdError;
use thiserror::Error;

use semver::Error as SemverError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}

impl From<SemverError> for ContractError {
    fn from(_: SemverError) -> Self {
        ContractError::Std(StdError::invalid_utf8("semver err"))
    }
}
