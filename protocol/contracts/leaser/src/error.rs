use std::{
    fmt::{Debug, Display},
    num::TryFromIntError,
};

use finance::percent::Percent;
use thiserror::Error;

use sdk::cosmwasm_std::StdError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Leaser] Save Config failed, cause: {0}")]
    SaveConfigFailure(StdError),

    #[error("[Leaser] Update Config failed, cause: {0}")]
    UpdateConfigFailure(StdError),

    #[error("[Leaser] Load Config failed, cause: {0}")]
    LoadConfigFailure(StdError),

    #[error("[Leaser] Loading the old Config failed, cause: {0}")]
    LoadOldConfig(StdError),

    #[error("[Leaser] Iterate Cutomer Leases failed, cause: {0}")]
    IterateLeasesFailure(StdError),

    #[error("[Leaser] Registration of a Lease failed, cause: {0}")]
    SaveLeaseFailure(StdError),

    #[error("[Leaser] Deregistration of a Lease failed, cause: {0}")]
    RemoveLeaseFailure(StdError),

    #[error("[Leaser] Load Customer Leases failed, cause: {0}")]
    LoadLeasesFailure(StdError),

    #[error("[Leaser] Save pending Customer failed, cause: {0}")]
    SavePendingCustomerFailure(StdError),

    #[error("[Leaser] Address validation failed, cause: {0}")]
    InvalidAddress(StdError),

    #[error("[Leaser] Failed to serialize to JSON, cause: {0}")]
    SerializeToJson(StdError),

    #[error("[Leaser] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),

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
    QueryLeasePackage(StdError),

    #[error("[Leaser] Overflow during `{0}`")]
    ComputationOverflow(String),
}

impl ContractError {
    pub fn overflow_price_total<C, P>(cause: &str, amount: C, price: P) -> Self
    where
        C: Display,
        P: Debug,
    {
        Self::ComputationOverflow(format!("`{cause}`. amount: {}, price: {:?}", amount, price))
    }

    pub fn overflow_init_borrow_amount<P>(
        cause: &str,
        downpayment: P,
        may_max_ltd: Option<Percent>,
    ) -> Self
    where
        P: Display,
    {
        Self::ComputationOverflow(format!(
            "`{cause}`. downpayment: {}, max_ltd: {:?}",
            downpayment, may_max_ltd
        ))
    }
}
