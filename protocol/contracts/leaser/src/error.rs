use std::num::TryFromIntError;

use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Leaser] Save Config failed, cause: {0}")]
    SaveConfigFailure(String),

    #[error("[Leaser] Update Config failed, cause: {0}")]
    UpdateConfigFailure(String),

    #[error("[Leaser] Load Config failed, cause: {0}")]
    LoadConfigFailure(String),

    // #[error("[Leaser] Loading the old Config failed, cause: {0}")]
    // LoadOldConfig(String),
    #[error("[Leaser] Iterate Cutomer Leases failed, cause: {0}")]
    IterateLeasesFailure(String),

    #[error("[Leaser] Registration of a Lease failed, cause: {0}")]
    SaveLeaseFailure(String),

    #[error("[Leaser] Deregistration of a Lease failed, cause: {0}")]
    RemoveLeaseFailure(String),

    #[error("[Leaser] Load Customer Leases failed, cause: {0}")]
    LoadLeasesFailure(String),

    #[error("[Leaser] Save pending Customer failed, cause: {0}")]
    SavePendingCustomerFailure(String),

    #[error("[Leaser] Address validation failed, cause: {0}")]
    InvalidAddress(String),

    #[error("[Leaser] Failed to serialize to JSON, cause: {0}")]
    SerializeToJson(String),

    #[error("[Leaser] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),

    #[error("[Leaser] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Lease] {0}")]
    LppStubCreation(lpp::stub::Error),

    #[error("[Lease] {0}")]
    QuoteQuery(lpp::stub::lender::Error),

    #[error("[Leaser] {0}")]
    CloseAllDeposits(lpp::stub::deposit::Error),

    #[error("[Leaser] {0}")]
    PriceOracle(#[from] oracle_platform::error::Error),

    #[error("[Leaser] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Leaser] Failed to update software! Cause: {0}")]
    UpdateSoftware(versioning::Error),

    #[error("[Leaser] {0}")]
    Unauthorized(#[from] access_control::error::Error),

    #[error("[Leaser] ParseError {err:?}")]
    Parsing { err: String },

    #[error("[Leaser] {0}")]
    Reserve(#[from] reserve::stub::Error),

    #[error("[Leaser] Cannot open lease with zero downpayment")]
    ZeroDownpayment {},

    #[error("[Leaser] No Liquidity")]
    NoLiquidity {},

    #[error("[Leaser] Invalid continuation key, cause: {err} ")]
    InvalidContinuationKey { err: String },

    #[error("[Leaser] The protocol is still in use. There are open leases")]
    ProtocolStillInUse(),

    #[error(
        "[Leaser][ProtocolsRegistry] The protocol deregistration request preparation failed! Cause: {0}"
    )]
    ProtocolDeregistration(platform::error::Error),

    #[error("[Leaser] Scheduling a reserve dump has failed! Cause: {0}")]
    ScheduleReserveDump(platform::error::Error),

    #[error("[Leaser] Failed to query for the Lease package, cause: {0}")]
    QueryLeasePackage(String),
}

impl ContractError {
    pub(crate) fn save_config_failure(error: StdError) -> Self {
        Self::SaveConfigFailure(error.to_string())
    }

    pub(crate) fn update_config_failure(error: StdError) -> Self {
        Self::UpdateConfigFailure(error.to_string())
    }

    pub(crate) fn load_config_failure(error: StdError) -> Self {
        Self::LoadConfigFailure(error.to_string())
    }

    // pub(crate) fn load_old_config(error: StdError) -> Self {
    //     Self::LoadOldConfig(error.to_string())
    // }

    pub(crate) fn iterate_leases_failure(error: StdError) -> Self {
        Self::IterateLeasesFailure(error.to_string())
    }

    pub(crate) fn save_lease_failure(error: StdError) -> Self {
        Self::SaveLeaseFailure(error.to_string())
    }

    pub(crate) fn remove_lease_failure(error: StdError) -> Self {
        Self::RemoveLeaseFailure(error.to_string())
    }

    pub(crate) fn load_leases_failure(error: StdError) -> Self {
        Self::LoadLeasesFailure(error.to_string())
    }

    pub(crate) fn save_pending_customer_failure(error: StdError) -> Self {
        Self::SavePendingCustomerFailure(error.to_string())
    }

    pub(crate) fn invalid_address(error: StdError) -> Self {
        Self::InvalidAddress(error.to_string())
    }

    pub(crate) fn serialize_to_json(error: StdError) -> Self {
        Self::SerializeToJson(error.to_string())
    }

    pub(crate) fn query_lease_package(error: StdError) -> Self {
        Self::QueryLeasePackage(error.to_string())
    }
}
