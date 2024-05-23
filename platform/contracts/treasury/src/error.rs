use std::convert::Infallible;

use thiserror::Error;

use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("[Dispatcher] Failed to serialize! Cause: {0}")]
    Serialize(StdError),

    #[error("[Dispatcher] Failed to init the contract version! Cause: {0}")]
    InitVersion(StdError),

    #[error("[Dispatcher] Failed to validate the Registry address! Cause: {0}")]
    ValidateRegistryAddr(StdError),

    #[error("[Dispatcher] Failed to update the storage! Cause: {0}")]
    UpdateStorage(StdError),

    #[error("[Dispatcher] Failed to update the software! Cause: {0}")]
    UpdateSoftware(StdError),

    #[error("[Dispatcher] Failed to retrieve all protocols! Cause: {0}")]
    QueryProtocols(StdError),

    #[error("[Dispatcher] Failed to protocol contracts! Cause: {0}")]
    QueryProtocol(StdError),

    #[error("[Dispatcher] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Dispatcher] Failed to load the configuration! Cause: {0}")]
    LoadConfig(StdError),

    #[error("[Dispatcher] Failed to save the configuration! Cause: {0}")]
    SaveConfig(StdError),

    #[error("[Dispatcher] Failed to load the dispatch log! Cause: {0}")]
    LoadDispatchLog(StdError),

    #[error("[Dispatcher] Failed to save the dispatch log! Cause: {0}")]
    SaveDispatchLog(StdError),

    #[error("[Dispatcher] Failed to obtain Lpp balance! Cause: {0}")]
    ReadLppBalance(lpp_platform::error::Error),

    #[error("[Dispatcher] Failed to distribute rewards to an Lpp! Cause: {0}")]
    DistributeLppReward(lpp_platform::error::Error),

    #[error("[Dispatcher] Failed to convert rewards to NLS! Cause: {0}")]
    ConvertRewardsToNLS(oracle_platform::error::Error),

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
}
