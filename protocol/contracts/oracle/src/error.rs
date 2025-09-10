use std::num::TryFromIntError;

use thiserror::Error;

#[cfg(feature = "contract")]
use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};

#[cfg(feature = "contract")]
use finance::price::dto::PriceDTO;
use marketprice::{alarms::errors::AlarmError, error::PriceFeedsError, feeders::PriceFeedersError};
use sdk::cosmwasm_std::{Addr, StdError};
use versioning::Error as VersioningError;

#[derive(Error, Debug, PartialEq)]
pub enum Error<PriceG>
where
    PriceG: Group,
{
    #[error("[Oracle] Failed to validate address while trying to register feeder! Cause: {0}")]
    RegisterFeederAddressValidation(String),

    #[error("[Oracle] Failed to validate address while trying to unregister feeder! Cause: {0}")]
    UnregisterFeederAddressValidation(String),

    #[error("[Oracle] Failed to update software! Cause: {0}")]
    UpdateSoftware(VersioningError),

    #[error("[Oracle] The configured swap tree does not conform to the code! Cause: {0}")]
    BrokenSwapTree(String),

    #[error("[Oracle] Failed to load feeders! Cause: {0}")]
    LoadFeeders(String),

    #[error("[Oracle] Failed to load configuration! Cause: {0}")]
    LoadConfig(String),

    #[error("[Oracle] Failed to update configuration! Cause: {0}")]
    UpdateConfig(String),

    #[error("[Oracle] Failed to store configuration! Cause: {0}")]
    StoreConfig(String),

    #[error("[Oracle] Failed to load supported pairs! Cause: {0}")]
    LoadSupportedPairs(String),

    #[error("[Oracle] Failed to store supported pairs! Cause: {0}")]
    StoreSupportedPairs(String),

    #[error("[Oracle] Failed to convert query response to binary! Cause: {0}")]
    ConvertToBinary(String),

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

    #[error("[Oracle][Base='{base}'] Invalid base currency '{invalid}'")]
    InvalidBaseCurrency {
        base: CurrencyDTO<PriceG>,
        invalid: CurrencyDTO<PriceG>,
    },

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
        base: CurrencyDTO<PriceG>,
        unsupported: CurrencyDTO<PriceG>,
    },

    #[error("[Oracle] integer conversion {0}")]
    Conversion(#[from] TryFromIntError),
}

impl<PriceG> Error<PriceG>
where
    PriceG: Group,
{
    pub(crate) fn register_feeder_address_validation(error: StdError) -> Self {
        Self::RegisterFeederAddressValidation(error.to_string())
    }

    pub(crate) fn unregister_feeder_address_validation(error: StdError) -> Self {
        Self::UnregisterFeederAddressValidation(error.to_string())
    }

    pub(crate) fn load_feeders(error: StdError) -> Self {
        Self::LoadFeeders(error.to_string())
    }

    pub(crate) fn load_config(error: StdError) -> Self {
        Self::LoadConfig(error.to_string())
    }

    pub(crate) fn update_config(error: StdError) -> Self {
        Self::UpdateConfig(error.to_string())
    }

    pub(crate) fn store_config(error: StdError) -> Self {
        Self::StoreConfig(error.to_string())
    }

    pub(crate) fn load_supported_pairs(error: StdError) -> Self {
        Self::LoadSupportedPairs(error.to_string())
    }

    pub(crate) fn store_supported_pairs(error: StdError) -> Self {
        Self::StoreSupportedPairs(error.to_string())
    }

    pub fn convert_to_binary(error: StdError) -> Self {
        Self::ConvertToBinary(error.to_string())
    }
}

#[cfg(feature = "contract")]
pub(crate) fn unsupported_currency<G, BaseC>(unsupported: CurrencyDTO<G>) -> Error<G>
where
    G: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<G>,
{
    Error::UnsupportedCurrency {
        base: BaseC::dto().into_super_group(),
        unsupported,
    }
}

#[cfg(feature = "contract")]
pub(crate) fn invalid_base_currency<G, BaseC>(configured_base: CurrencyDTO<G>) -> Error<G>
where
    G: Group,
    BaseC: CurrencyDef,
    BaseC::Group: MemberOf<G>,
{
    Error::InvalidBaseCurrency {
        base: BaseC::dto().into_super_group(),
        invalid: configured_base,
    }
}

#[cfg(feature = "contract")]
pub(crate) fn unsupported_denom_pairs<G>(price: &PriceDTO<G>) -> Error<G>
where
    G: Group,
{
    Error::UnsupportedDenomPairs(price.to_string())
}
