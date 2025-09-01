use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("[Treasury] [Std] {0}")]
    Std(String),

    #[error("[Treasury] {0}")]
    Versioning(#[from] versioning::Error),

    #[error("[Treasury] Failed to serialize! Cause: {0}")]
    Serialize(String),

    #[error("[Treasury] Failed to init the contract version! Cause: {0}")]
    InitVersion(String),

    #[error("[Treasury] Failed to validate the Registry address! Cause: {0}")]
    ValidateRegistryAddr(String),

    #[error("[Treasury] Failed to update the storage! Cause: {0}")]
    UpdateStorage(String),

    #[error("[Treasury] Failed to update the software! Cause: {0}")]
    UpdateSoftware(String),

    #[error("[Treasury] Failed to retrieve all protocols! Cause: {0}")]
    QueryProtocols(String),

    #[error("[Treasury] Failed to protocol contracts! Cause: {0}")]
    QueryProtocol(String),

    #[error("[Treasury] {0}")]
    SerializeResponse(#[from] platform::error::Error),

    #[error("[Treasury] Failed to load the configuration! Cause: {0}")]
    LoadConfig(String),

    #[error("[Treasury] Failed to save the configuration! Cause: {0}")]
    SaveConfig(String),

    #[error("[Treasury] Failed to load the dispatch log! Cause: {0}")]
    LoadDispatchLog(String),

    #[error("[Treasury] Failed to save the dispatch log! Cause: {0}")]
    SaveDispatchLog(String),

    #[error("[Treasury] Failed to obtain Lpp balance! Cause: {0}")]
    ReadLppBalance(lpp_platform::error::Error),

    #[error("[Treasury] Failed to distribute rewards to an Lpp! Cause: {0}")]
    DistributeLppReward(lpp_platform::error::Error),

    #[error("[Treasury] Failed to convert rewards to NLS! Cause: {0}")]
    ConvertRewardsToNLS(oracle_platform::error::Error),

    #[error("[Treasury] Failed to setup a time alarms stub! Cause: {0}")]
    SetupTimeAlarmStub(timealarms::stub::Error),

    #[error("[Treasury] Failed to setup a time alarm! Cause: {0}")]
    SetupTimeAlarm(timealarms::stub::Error),

    #[error("[Treasury] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error(
        "[Treasury] Invalid time configuration. Current reward distribution time is before the last distribution time"
    )]
    InvalidTimeConfiguration {},
}
