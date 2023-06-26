use std::num::TryFromIntError;

use thiserror::Error;

use currency::{Currency, Symbol, SymbolOwned};
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};
use sdk::cosmwasm_std::{Addr, StdError};

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[cfg(feature = "testing")]
    #[error("[Oracle] [Std] {0}")]
    Std(#[from] StdError),

    #[error("[Oracle; Stub] Failed to query configuration! Context: {0}")]
    StubConfigQuery(StdError),

    #[error("[Oracle; Stub] Failed to query swap path! Context: {0}")]
    StubSwapPathQuery(StdError),

    #[error("[Oracle; Stub] Failed to add alarm! Context: {0}")]
    StubAddAlarm(StdError),

    #[error("[Oracle] Failed to initialize versioning module! Context: {0}")]
    InitializeVersioning(StdError),

    #[error("[Oracle] Failed to validate address while trying to register feeder! Context: {0}")]
    RegisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to validate address while trying to unregister feeder! Context: {0}")]
    UnregisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to update software! Context: {0}")]
    UpdateSoftware(StdError),

    #[error("[Oracle] Failed to load feeders! Context: {0}")]
    LoadFeeders(StdError),

    #[error("[Oracle] Failed to load configuration! Context: {0}")]
    LoadConfig(StdError),

    #[error("[Oracle] Failed to update configuration! Context: {0}")]
    UpdateConfig(StdError),

    #[error("[Oracle] Failed to store configuration! Context: {0}")]
    StoreConfig(StdError),

    #[error("[Oracle] Failed to load supported pairs! Context: {0}")]
    LoadSupportedPairs(StdError),

    #[error("[Oracle] Failed to store supported pairs! Context: {0}")]
    StoreSupportedPairs(StdError),

    #[error("[Oracle] Failed to convert query response to binary! Context: {0}")]
    ConvertToBinary(StdError),

    #[error("[Oracle] {0}")]
    PriceFeedersError(#[from] PriceFeedersError),

    #[error("[Oracle] {0}")]
    PriceFeedsError(#[from] PriceFeedsError),

    #[error("[Oracle] {0}")]
    AlarmError(#[from] AlarmError),

    #[error("[Oracle] {0}")]
    Currency(#[from] currency::error::Error),

    #[error("[Oracle] {0}")]
    Finance(#[from] finance::error::Error),

    #[error("[Oracle] Unsupported denom pairs")]
    UnsupportedDenomPairs {},

    #[error("[Oracle] Invalid feeder address")]
    InvalidAddress {},

    #[error("[Oracle] Failed to fetch price for the pair {from}/{to}! Possibly no price is available! Cause: {error}")]
    FailedToFetchPrice {
        from: SymbolOwned,
        to: SymbolOwned,
        error: StdError,
    },

    #[error("[Oracle] Invalid denom pair ({0}, {1})")]
    InvalidDenomPair(SymbolOwned, SymbolOwned),

    #[error("[Oracle] Invalid base currency ({0} != {1})")]
    InvalidBaseCurrency(SymbolOwned, SymbolOwned),

    #[error("[Oracle] Duplicated nodes in the currency tree")]
    DuplicatedNodes {},

    #[error("[Oracle] No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("[Oracle] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Oracle] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("Mismatch of curencies, expected {expected:?}, found {found:?}")]
    CurrencyMismatch { expected: String, found: String },

    #[error("[Oracle][Base='{base}'] Unsupported currency '{unsupported}'")]
    UnsupportedCurrency {
        base: SymbolOwned,
        unsupported: SymbolOwned,
    },

    #[error("[Oracle] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),
}

pub fn currency_mismatch<ExpC>(found: SymbolOwned) -> ContractError
where
    ExpC: Currency,
{
    ContractError::CurrencyMismatch {
        expected: ExpC::TICKER.into(),
        found,
    }
}

pub fn unsupported_currency<C>(unsupported: Symbol<'_>) -> ContractError
where
    C: Currency,
{
    ContractError::UnsupportedCurrency {
        base: C::TICKER.into(),
        unsupported: unsupported.into(),
    }
}
