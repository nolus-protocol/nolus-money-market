use std::{num::TryFromIntError, result::Result as StdResult};

use thiserror::Error;

use currency::SymbolOwned;
#[cfg(feature = "contract")]
use currency::{Currency, SymbolSlice};
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};
use sdk::cosmwasm_std::{Addr, StdError};

//TODO migrate to the same type defined at oracle::result
pub type Result<T> = StdResult<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Oracle] Failed to initialize versioning module! Cause: {0}")]
    InitializeVersioning(StdError),

    #[error("[Oracle] Failed to validate address while trying to register feeder! Cause: {0}")]
    RegisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to validate address while trying to unregister feeder! Cause: {0}")]
    UnregisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to update software! Cause: {0}")]
    UpdateSoftware(StdError),

    #[error("[Oracle] Failed to load feeders! Cause: {0}")]
    LoadFeeders(StdError),

    #[error("[Oracle] Failed to load configuration! Cause: {0}")]
    LoadConfig(StdError),

    #[error("[Oracle] Failed to update configuration! Cause: {0}")]
    UpdateConfig(StdError),

    #[error("[Oracle] Failed to store configuration! Cause: {0}")]
    StoreConfig(StdError),

    #[error("[Oracle] Failed to load the v0 configuration! Cause: {0}")]
    LoadConfigV0(StdError),

    #[error("[Oracle] Failed to load supported pairs! Cause: {0}")]
    LoadSupportedPairs(StdError),

    #[error("[Oracle] Failed to store supported pairs! Cause: {0}")]
    StoreSupportedPairs(StdError),

    #[error("[Oracle] Failed to convert query response to binary! Cause: {0}")]
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

    #[error("[Oracle] Invalid denom pair ({0}, {1})")]
    InvalidDenomPair(SymbolOwned, SymbolOwned),

    #[error("[Oracle] Invalid base currency ({0} != {1})")]
    InvalidBaseCurrency(SymbolOwned, SymbolOwned),

    #[error("[Oracle] Specified stable currency is not in the currency tree")]
    StableCurrencyNotInTree {},

    #[error("[Oracle] Duplicated nodes in the currency tree")]
    DuplicatedNodes {},

    #[error("[Oracle] No feeder data for the specified address")]
    UnknownFeeder {},

    #[error("[Oracle] Invalid alarm notification address: {0:?}")]
    InvalidAlarmAddress(Addr),

    #[error("[Oracle] {0}")]
    Platform(#[from] platform::error::Error),

    #[error("[Oracle][Base='{base}'] Unsupported currency '{unsupported}'")]
    UnsupportedCurrency {
        base: SymbolOwned,
        unsupported: SymbolOwned,
    },

    #[error("[Oracle] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),
}

#[cfg(feature = "contract")]
pub(crate) fn unsupported_currency<C>(unsupported: &SymbolSlice) -> ContractError
where
    C: Currency,
{
    ContractError::UnsupportedCurrency {
        base: C::TICKER.into(),
        unsupported: unsupported.into(),
    }
}
