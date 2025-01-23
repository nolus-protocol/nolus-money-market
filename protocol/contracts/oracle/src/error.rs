use std::{num::TryFromIntError, result::Result as StdResult};

use thiserror::Error;

#[cfg(feature = "contract")]
use currency::{CurrencyDTO, CurrencyDef, Group};
use currency::{SymbolOwned, SymbolStatic};
#[cfg(feature = "contract")]
use finance::price::dto::PriceDTO;
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};
use sdk::cosmwasm_std::{Addr, StdError};
use versioning::Error as VersioningError;

//TODO migrate to the same type defined at oracle::result
pub type Result<T> = StdResult<T, ContractError>;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("[Oracle] Failed to validate address while trying to register feeder! Cause: {0}")]
    RegisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to validate address while trying to unregister feeder! Cause: {0}")]
    UnregisterFeederAddressValidation(StdError),

    #[error("[Oracle] Failed to update software! Cause: {0}")]
    UpdateSoftware(VersioningError),

    #[error("[Oracle] The configured swap tree does not conform to the code! Cause: {0}")]
    BrokenSwapTree(String),

    #[error("[Oracle] Failed to load feeders! Cause: {0}")]
    LoadFeeders(StdError),

    #[error("[Oracle] Failed to load configuration! Cause: {0}")]
    LoadConfig(StdError),

    #[error("[Oracle] Failed to update configuration! Cause: {0}")]
    UpdateConfig(StdError),

    #[error("[Oracle] Failed to store configuration! Cause: {0}")]
    StoreConfig(StdError),

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

    #[error("[Oracle] Unsupported price {0}")]
    UnsupportedDenomPairs(String),

    #[error("[Oracle] Invalid feeder address")]
    InvalidAddress {},

    #[error("[Oracle] Invalid denom pair ({0}, {1})")]
    InvalidDenomPair(SymbolOwned, SymbolOwned),

    #[error("[Oracle][Base='{0}'] Invalid base currency '{1}'")]
    InvalidBaseCurrency(SymbolStatic, SymbolOwned),

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
        base: SymbolStatic,
        unsupported: SymbolOwned,
    },

    #[error("[Oracle] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),
}

#[cfg(feature = "contract")]
pub(crate) fn unsupported_currency<G, BaseC>(unsupported: &CurrencyDTO<G>) -> ContractError
where
    G: Group,
    BaseC: CurrencyDef,
{
    ContractError::UnsupportedCurrency {
        base: currency::to_string::<BaseC>(BaseC::definition()),
        unsupported: unsupported.to_string(),
    }
}

#[cfg(feature = "contract")]
pub(crate) fn invalid_base_currency<G, BaseC>(configured_base: &CurrencyDTO<G>) -> ContractError
where
    G: Group,
    BaseC: CurrencyDef,
{
    ContractError::InvalidBaseCurrency(
        currency::to_string::<BaseC>(BaseC::definition()),
        configured_base.to_string(),
    )
}

#[cfg(feature = "contract")]
pub(crate) fn unsupported_denom_pairs<G>(price: &PriceDTO<G>) -> ContractError
where
    G: Group,
{
    ContractError::UnsupportedDenomPairs(price.to_string())
}
