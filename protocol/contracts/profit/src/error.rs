use thiserror::Error;

use sdk::cosmwasm_std::{Addr, Instantiate2AddressError, StdError};

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("[Profit] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Profit] [Std] [Instantiate2] {0}")]
    StdInstantiate2Addr(#[from] Instantiate2AddressError),

    #[error("[Profit] {0}")]
    Dex(#[from] dex::Error),

    #[error("[Profit] remote-profit wire: {0}")]
    RemoteProfit(#[from] remote_profit_wire::error::Error),

    #[error("[Profit] {0}")]
    PriceOracle(#[from] oracle_platform::error::Error),

    #[error("[Profit] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Profit] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error(
        "[Profit] Migration is not supported. The remote-swap profit is deployed fresh, never migrated from the ICA profit."
    )]
    MigrationUnsupported,

    #[error(
        "[Profit] The instantiated drain-vault address {reported} differs from the precomputed {expected}"
    )]
    DifferentInstantiatedAddress { reported: Addr, expected: Addr },

    #[error(
        "[Profit] No funding cycle can start before the Solana profit authority is learned from the open-profit acknowledgment"
    )]
    SolanaAuthorityNotLearned,

    #[error("[Profit] {0}")]
    TimeAlarm(#[from] timealarms::stub::Error),

    #[error("[Profit] Invalid contract address {0}")]
    InvalidContractAddress(Addr),

    #[error("[Profit] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Profit] Alarm comming from unknown address: {0:?}")]
    UnrecognisedAlarm(Addr),

    #[error("[Profit] Operation is not supported at this time. Cause: {0}")]
    UnsupportedOperation(String),

    #[error(
        "[Profit] Invalid time configuration. Current profit transfer time is before the last transfer time"
    )]
    InvalidTimeConfiguration {},

    #[error("[Profit] EmptyBalance. No profit to dispatch")]
    EmptyBalance {},
}

impl ContractError {
    pub(crate) fn unsupported_operation(msg: &'static str) -> Self {
        Self::UnsupportedOperation(String::from(msg))
    }
}
