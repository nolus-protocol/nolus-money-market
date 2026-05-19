use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[RemoteLease] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[RemoteLease] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[RemoteLease] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[RemoteLease] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[RemoteLease] {0}")]
    RemoteCallback(#[from] remote_lease::error::Error),

    #[error("[RemoteLease] Caller is not an authorised Lease instance")]
    UnauthorisedCaller,

    #[error("[RemoteLease] {0} must be non-empty")]
    EmptyInstantiateField(&'static str),

    #[error("[RemoteLease] A channel is already recorded for this controller")]
    ChannelAlreadyExists,

    #[error("[RemoteLease] No channel is recorded for this controller")]
    ChannelNotOpen,

    #[error("[RemoteLease] The recorded channel is not operational")]
    ChannelNotOperational,

    #[error("[RemoteLease] Counterparty port id mismatch: expected '{expected}', got '{actual}'")]
    InvalidCounterpartyPort { expected: String, actual: String },

    #[error("[RemoteLease] Channel version mismatch: expected '{expected}', got '{actual}'")]
    InvalidChannelVersion { expected: String, actual: String },

    #[error("[RemoteLease] Channel ordering must be UNORDERED")]
    InvalidChannelOrdering,

    #[error("[RemoteLease] Channel connection id mismatch: expected '{expected}', got '{actual}'")]
    InvalidConnectionId { expected: String, actual: String },

    #[error("[RemoteLease] Counterparty-initiated channel-open handshakes are not supported")]
    UnsupportedCounterpartyOpen,

    #[error("[RemoteLease] Unsolicited channel close attempt rejected")]
    UnsolicitedChannelClose,

    #[error("[RemoteLease] Inbound packets are not supported by this controller")]
    UnsupportedInboundPacket,
}

pub type Result<T> = std::result::Result<T, Error>;
