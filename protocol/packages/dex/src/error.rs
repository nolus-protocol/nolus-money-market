use std::fmt::Display;

use thiserror::Error;

use crate::{swap, swap_task::CoinsNb};

#[derive(Error, Debug)]
pub enum Error {
    #[error("[Dex] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dex] {0}")]
    Swap(#[from] swap::Error),

    #[error("[Dex] The operation '{0}' is not supported in the current state '{1}'")]
    UnsupportedOperation(String, String),

    #[error("[Dex] {0}")]
    OracleSwapError(#[from] oracle::api::swap::Error),

    #[error("[Dex] {0}")]
    MinOutput(oracle::stub::Error),

    #[error("[Dex] {0}")]
    TimeAlarmError(#[from] timealarms::stub::Error),

    #[error("[Dex] {0}")]
    Unauthorized(access_control::error::Error),

    #[error("[Dex] [RemoteSwap] {0}")]
    RemoteSwapClient(String),

    #[error("[Dex] [RemoteSwap] {0}")]
    UnexpectedResponseVariant(String),

    #[error("[Dex] [RemoteSwap] No in-flight swap leg matches the current task state")]
    MissingSwapLeg,

    #[error("[Dex] [RemoteSwap] The number of swap legs exceeds the supported maximum of {0}")]
    SwapLegsNbOverflow(CoinsNb),

    #[error("[Dex] [RemoteTransferOut] No in-flight transfer matches the current task state")]
    MissingTransferOutLeg,

    #[error(
        "[Dex] [RemoteTransferOut] The number of transfers exceeds the supported maximum of {0}"
    )]
    TransferOutLegsNbOverflow(CoinsNb),
}

pub type Result<T> = core::result::Result<T, Error>;

impl Error {
    /// Wrap a transport-specific failure of a [`RemoteSwapClient`][crate::RemoteSwapClient]
    /// implementation
    pub fn remote_swap_client<Details>(details: Details) -> Self
    where
        Details: Display,
    {
        Self::RemoteSwapClient(details.to_string())
    }

    /// Wrap a response that decodes correctly yet does not carry the
    /// scheduled swap's result - protocol confusion, as opposed to the
    /// wire garbage [`remote_swap_client`][Self::remote_swap_client] wraps
    pub fn unexpected_response_variant<Details>(details: Details) -> Self
    where
        Details: Display,
    {
        Self::UnexpectedResponseVariant(details.to_string())
    }

    pub(crate) fn unsupported_operation<Op, State>(op: Op, state: State) -> Self
    where
        Op: Into<String>,
        State: std::fmt::Display,
    {
        Self::UnsupportedOperation(op.into(), format!("{state}"))
    }
}
