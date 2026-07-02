use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum Error {
    #[error(
        "the three remote-lease currencies (downpayment, lpn, asset) must be pairwise distinct"
    )]
    DuplicateLeaseCurrencies,

    #[error("swap input and output currencies must differ")]
    SameSwapCurrency,

    #[error("swap input amount and minimum output must be greater than zero")]
    ZeroSwapAmount,

    #[error("transfer-out amount must be greater than zero")]
    ZeroTransferAmount,

    #[error("callback error message exceeds the {max}-byte cap (was {actual})")]
    CallbackErrorTooLong { actual: usize, max: usize },

    #[error("remote-lease-id must not be empty")]
    RemoteLeaseIdEmpty,

    #[error("remote-lease-id exceeds the {max}-byte cap (was {actual})")]
    RemoteLeaseIdTooLong { actual: usize, max: usize },

    #[error("remote-lease-id contains a non-base58 byte 0x{byte:02x}")]
    RemoteLeaseIdInvalidCharacter { byte: u8 },
}
