use semver::Error as SemverError;
use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("[Treasury] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Treasury] {0}")]
    PlatformError(#[from] platform::error::Error),

    #[error("[Treasury] {0}")]
    Unauthorized(#[from] platform::access_control::Unauthorized),

    #[error("[Treasury] Rewards dispatcher is not configured")]
    NotConfigured {},
}

impl From<SemverError> for ContractError {
    fn from(_: SemverError) -> Self {
        ContractError::Std(StdError::invalid_utf8("semver err"))
    }
}
