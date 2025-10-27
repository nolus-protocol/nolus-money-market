use thiserror::Error as ThisError;

use platform::contract::CodeId;
use sdk::cosmwasm_std::{Addr, StdError};
use versioning::ReleaseId;

#[derive(Debug, PartialEq, ThisError)]
pub enum Error {
    #[error("[Admin] [Std] {0}")]
    StdError(#[from] StdError),
    #[error("[Admin] [Std] [Instantiate2] {0}")]
    StdInstantiate2Addr(#[from] sdk::cosmwasm_std::Instantiate2AddressError),
    #[error("[Admin] {0}")]
    AccessControl(#[from] access_control::error::Error),
    #[error("[Admin] {0}")]
    Platform(#[from] platform::error::Error),
    #[error("[Admin] {0}")]
    Versioning(#[from] versioning::Error),
    #[error("[Admin] No data in migration response!")]
    NoMigrationResponseData {},
    #[error(
        "[Admin] Contract returned wrong release string! \"{reported}\" was \
        returned, but \"{expected}\" was expected!"
    )]
    WrongRelease {
        reported: ReleaseId,
        expected: ReleaseId,
    },
    #[error(
        "[Admin] Contract returned wrong address! Expected \"{expected}\", \
        but got \"{reported}\"!"
    )]
    DifferentInstantiatedAddress { reported: Addr, expected: Addr },
    #[error(
        "[Admin] Contract returned wrong code id! Expected \"{expected}\", \
        but got \"{reported}\"!"
    )]
    DifferentInstantiatedCodeId { reported: CodeId, expected: CodeId },
    #[error(
        "[Admin] Protocol not mentioned under either migration messages, or \
        post-migration execution messages! Protocol's friendly name: {0}"
    )]
    MissingProtocol(String),
    #[error("[Admin] Failed to load the configuration! Cause: {0}")]
    LoadConfig(StdError),
    #[error(
        "[Admin] Protocol deregistration message not sent by a registered \
        protocol leaser!"
    )]
    SenderNotARegisteredLeaser {},
    #[error(
        "[Admin] Protocol set of contracts already exists for this protocol \
        name! Protocol's friendly name: {0}"
    )]
    ProtocolSetAlreadyExists(String),
    #[error(
        "[Admin] No protocol set of contracts exists for this protocol name! \
        Protocol's friendly name: {0}"
    )]
    UnknownProtocol(String),
}
