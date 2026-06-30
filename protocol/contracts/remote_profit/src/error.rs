use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("[RemoteProfit] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[RemoteProfit] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[RemoteProfit] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[RemoteProfit] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[RemoteProfit] {0}")]
    RemoteCallback(#[from] remote_profit::error::Error),

    #[error("[RemoteProfit] Caller is not an authorised Profit instance")]
    UnauthorisedCaller,

    #[error("[RemoteProfit] {0} must be non-empty")]
    EmptyInstantiateField(&'static str),

    #[error(
        "[RemoteProfit] The stored config does not deserialize under the current schema, cause: {0}"
    )]
    IncompatibleStoredConfig(StdError),

    #[error("[RemoteProfit] The stored config violates its invariant")]
    MalformedStoredConfig,

    #[error("[RemoteProfit] Transfer channel id '{0}' is not a canonical 'channel-<N>' identifier")]
    NonCanonicalTransferChannel(String),

    #[error(
        "[RemoteProfit] Counterparty channel version mismatch: expected '{expected}', got '{actual}'"
    )]
    InvalidCounterpartyVersion { expected: String, actual: String },

    #[error("[RemoteProfit] A channel is already recorded for this controller")]
    ChannelAlreadyExists,

    #[error("[RemoteProfit] No channel is recorded for this controller")]
    ChannelNotOpen,

    #[error("[RemoteProfit] The recorded channel is not operational")]
    ChannelNotOperational,

    #[error("[RemoteProfit] Counterparty port id mismatch: expected '{expected}', got '{actual}'")]
    InvalidCounterpartyPort { expected: String, actual: String },

    #[error("[RemoteProfit] Channel version mismatch: expected '{expected}', got '{actual}'")]
    InvalidChannelVersion { expected: String, actual: String },

    #[error("[RemoteProfit] Channel ordering must be UNORDERED")]
    InvalidChannelOrdering,

    #[error("[RemoteProfit] Channel connection id mismatch: expected '{expected}', got '{actual}'")]
    InvalidConnectionId { expected: String, actual: String },

    #[error("[RemoteProfit] Counterparty-initiated channel-open handshakes are not supported")]
    UnsupportedCounterpartyOpen,

    #[error("[RemoteProfit] Unsolicited channel close attempt rejected")]
    UnsolicitedChannelClose,

    #[error("[RemoteProfit] Inbound packets are not supported by this controller")]
    UnsupportedInboundPacket,
}

pub type Result<T> = std::result::Result<T, Error>;
