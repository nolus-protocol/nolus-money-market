use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, PartialEq, Debug)]
pub enum ContractError {
    #[error("[Treasury] [Std] {0}")]
    Std(String),

    #[error("[Treasury] {0}")]
    Versioning(#[from] versioning::Error),

    #[error("[Treasury] Failed to serialize! Cause: {0}")]
    Serialize(String),

    // #[error("[Treasury] Failed to init the contract version! Cause: {0}")]
    // InitVersion(String),
    #[error("[Treasury] Failed to validate the Registry address! Cause: {0}")]
    ValidateRegistryAddr(String),

    #[error("[Treasury] Failed to update the storage! Cause: {0}")]
    UpdateStorage(String),

    // #[error("[Treasury] Failed to update the software! Cause: {0}")]
    // UpdateSoftware(String),
    #[error("[Treasury] Failed to retrieve all protocols! Cause: {0}")]
    QueryProtocols(String),

    // #[error("[Treasury] Failed to protocol contracts! Cause: {0}")]
    // QueryProtocol(String),
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

    #[error("[Treasury] Failed to query the oracle for its stable ticker! Cause: {0}")]
    QueryStableTicker(oracle_platform::error::Error),

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

impl ContractError {
    pub(crate) fn serialize(error: StdError) -> Self {
        Self::Serialize(error.to_string())
    }

    // pub(crate) fn init_version(error: StdError) -> Self {
    //     Self::InitVersion(error.to_string())
    // }

    pub(crate) fn validate_registry_addr(error: StdError) -> Self {
        Self::ValidateRegistryAddr(error.to_string())
    }

    pub(crate) fn update_storage(error: StdError) -> Self {
        Self::UpdateStorage(error.to_string())
    }

    // pub(crate) fn update_software(error: StdError) -> Self {
    //     Self::UpdateSoftware(error.to_string())
    // }

    pub(crate) fn query_protocols(error: StdError) -> Self {
        Self::QueryProtocols(error.to_string())
    }

    // pub(crate) fn query_protocol(error: StdError) -> Self {
    //     Self::QueryProtocol(error.to_string())
    // }

    pub(crate) fn load_config(error: StdError) -> Self {
        Self::LoadConfig(error.to_string())
    }

    pub(crate) fn save_config(error: StdError) -> Self {
        Self::SaveConfig(error.to_string())
    }

    pub(crate) fn load_dispatch_log(error: StdError) -> Self {
        Self::LoadDispatchLog(error.to_string())
    }

    pub(crate) fn save_dispatch_log(error: StdError) -> Self {
        Self::SaveDispatchLog(error.to_string())
    }
}

impl From<StdError> for ContractError {
    fn from(value: StdError) -> Self {
        Self::Std(value.to_string())
    }
}
