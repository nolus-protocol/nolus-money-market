use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("swap input and output currencies must differ")]
    SameSwapCurrency,

    #[error("swap input amount and minimum output must be greater than zero")]
    ZeroSwapAmount,

    #[error("transfer-out amount must be greater than zero")]
    ZeroTransferAmount,

    #[error("callback error message exceeds the {max}-byte cap (was {actual})")]
    CallbackErrorTooLong { actual: usize, max: usize },

    #[error("remote-profit-id must not be empty")]
    RemoteProfitIdEmpty,

    #[error("remote-profit-id exceeds the {max}-byte cap (was {actual})")]
    RemoteProfitIdTooLong { actual: usize, max: usize },

    #[error("remote-profit-id contains a non-base58 byte 0x{byte:02x}")]
    RemoteProfitIdInvalidCharacter { byte: u8 },

    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    ProtocolVersionMismatch {
        expected: &'static str,
        actual: String,
    },
}
