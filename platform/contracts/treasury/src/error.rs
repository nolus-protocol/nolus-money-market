use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("[Treasury] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Treasury] {0}")]
    PlatformError(#[from] platform::error::Error),

    #[error("[Treasury] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Treasury] Rewards dispatcher is not configured")]
    NotConfigured {},
}
