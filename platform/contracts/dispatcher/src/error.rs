use std::convert::Infallible;

use thiserror::Error;

use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("[Dispatcher] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Dispatcher] {0}")]
    LppPlatformError(#[from] lpp_platform::error::Error),

    #[error("[Dispatcher] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dispatcher] {0}")]
    Oracle(#[from] oracle::ContractError),

    #[error("[Dispatcher] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Dispatcher] {0}")]
    TimeAlarm(#[from] timealarms::error::ContractError),

    #[error("[Dispatcher] [Infallible] {0}")]
    FromInfallible(#[from] Infallible),

    #[error("[Dispatcher] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Dispatcher] Unknown currency symbol: {symbol:?}")]
    UnknownCurrency { symbol: String },

    #[error("[Dispatcher] Invalid contract address {0}")]
    InvalidContractAddress(Addr),

    #[error("[Dispatcher] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Dispatcher] Alarm comming from unknown address: {0:?}")]
    UnrecognisedAlarm(Addr),

    #[error("[Dispatcher] Invalid time configuration. Current reward distribution time is before the last distribution time")]
    InvalidTimeConfiguration {},

    #[error("[Dispatcher] Zero Reward")]
    ZeroReward {},
}
