use thiserror::Error as ThisError;

use sdk::cosmwasm_std::StdError;

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("[Admin] [Std] {0}")]
    StdError(#[from] StdError),
    #[error("[Admin] {0}")]
    AccessControl(#[from] access_control::error::Error),
    #[error("[Admin] {0}")]
    Platform(#[from] platform::error::Error),
    #[error("No data in migration response!")]
    NoMigrationResponseData {},
    #[error("Contract returned wrong release string! \"{reported}\" was returned, but \"{expected}\" was expected!")]
    WrongRelease { reported: String, expected: String },
    #[error("Dex not mentioned under either migration messages, or post-migration execution messages! Dex's friendly name: {0}")]
    MissingDex(String),
    #[error(
        "Dex-bound set of contracts already exists for this dex name! Dex's friendly name: {0}"
    )]
    DexSetAlreadyExists(String),
    #[error("No dex-bound set of contracts exists for this dex name! Dex's friendly name: {0}")]
    UnknownDex(String),
}
