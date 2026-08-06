use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(
        "the three remote-lease currencies (downpayment, lpn, asset) must be pairwise distinct"
    )]
    DuplicateLeaseCurrencies,

    #[error("swap input and output currencies must differ")]
    SameSwapCurrency,

    #[error("the two swap input currencies must differ")]
    DuplicateSwapInputCurrency,

    #[error("swap input amount and minimum output must be greater than zero")]
    ZeroSwapAmount,

    #[error("transfer-out amount must be greater than zero")]
    ZeroTransferAmount,

    #[error("callback error message exceeds the {max}-byte cap (was {actual})")]
    CallbackErrorTooLong { actual: usize, max: usize },

    #[error(
        "callback error acknowledgement is missing its leading '[<code>]' frame, or the code is empty, over {max} bytes, or not lower-case alphanumeric"
    )]
    CallbackErrorCodeMissing { max: usize },

    /// Bounded by construction: the parser only reads a code of at most
    /// [`crate::callback::REMOTE_ERROR_CODE_MAX_BYTES`], so retaining it here
    /// cannot echo an unbounded counterparty string.
    #[error("callback error acknowledgement carries the unrecognised code '{code}'")]
    CallbackErrorCodeUnknown { code: String },

    #[error("remote-lease-id must not be empty")]
    RemoteLeaseIdEmpty,

    #[error("remote-lease-id exceeds the {max}-byte cap (was {actual})")]
    RemoteLeaseIdTooLong { actual: usize, max: usize },

    #[error("remote-lease-id contains a non-base58 byte 0x{byte:02x}")]
    RemoteLeaseIdInvalidCharacter { byte: u8 },

    #[error(
        "the paired ICS-20 channel id must read 'channel-<n>' with a canonical, u16-bounded ordinal"
    )]
    Ics20ChannelIdNonCanonical,

    #[error("the paired ICS-20 channel id exceeds the {max}-byte cap (was {actual})")]
    Ics20ChannelIdTooLong { actual: usize, max: usize },

    /// Deliberately payload-free: the offending value is a counterparty string,
    /// and retaining it would echo an input of unbounded length. A caller that
    /// needs the value for diagnostics bounds it itself through
    /// [`crate::channel_version::bounded_channel_version`].
    #[error("the channel handshake version must read '<protocol version>+transfer=channel-<n>'")]
    ChannelVersionMalformed,

    #[error("the channel handshake version exceeds the {max}-byte cap (was {actual})")]
    ChannelVersionTooLong { actual: usize, max: usize },

    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch {
        expected: &'static str,
        actual: String,
    },
}
