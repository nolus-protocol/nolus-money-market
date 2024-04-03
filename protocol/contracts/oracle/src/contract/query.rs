use currency::SymbolSlice;
use sdk::cosmwasm_std::{to_json_binary, Addr, Binary, Storage, Timestamp};
use versioning::package_version;

use crate::{
    api::{PriceCurrencies, PricesResponse, SwapTreeResponse},
    error::ContractError,
    result::ContractResult,
    state::supported_pairs::SupportedPairs,
};

use super::{config, oracle::feeder::Feeders, oracle::Oracle};

pub fn contract_version() -> ContractResult<Binary> {
    to_json_binary(&package_version!()).map_err(ContractError::ConvertToBinary)
}

pub fn config(storage: &dyn Storage) -> ContractResult<Binary> {
    config::query_config(storage)
        .and_then(|config| to_json_binary(&config).map_err(ContractError::ConvertToBinary))
}

pub fn swap_tree(storage: &dyn Storage) -> ContractResult<Binary> {
    SupportedPairs::load(storage)
        .map(|supported_pairs| supported_pairs.query_swap_tree().into_human_readable())
        .and_then(|tree| {
            to_json_binary(&SwapTreeResponse { tree }).map_err(ContractError::ConvertToBinary)
        })
}

pub fn feeders(storage: &dyn Storage) -> ContractResult<Binary> {
    Feeders::get(storage)
        .map_err(ContractError::LoadFeeders)
        .and_then(|feeders| to_json_binary(&feeders).map_err(ContractError::ConvertToBinary))
}

pub fn is_feeder(storage: &dyn Storage, address: &Addr) -> ContractResult<Binary> {
    Feeders::is_feeder(storage, address)
        .map_err(ContractError::LoadFeeders)
        .and_then(|is_feeder| to_json_binary(&is_feeder).map_err(ContractError::ConvertToBinary))
}

pub fn prices(storage: &dyn Storage, now: Timestamp) -> ContractResult<Binary> {
    Oracle::<'_, _, PriceCurrencies>::load(storage)
        .and_then(|oracle| oracle.try_query_prices(now))
        .map(|prices| PricesResponse { prices })
        .and_then(|response| to_json_binary(&response).map_err(ContractError::ConvertToBinary))
}

pub fn price(
    storage: &dyn Storage,
    now: Timestamp,
    currency: &SymbolSlice,
) -> ContractResult<Binary> {
    Oracle::<'_, _, PriceCurrencies>::load(storage)
        .and_then(|oracle| oracle.try_query_price(now, currency))
        .and_then(|price_dto| to_json_binary(&price_dto).map_err(ContractError::ConvertToBinary))
}

pub fn stable_currency(storage: &dyn Storage) -> ContractResult<Binary> {
    SupportedPairs::load(storage).and_then(|supported_pairs| {
        to_json_binary(supported_pairs.stable_currency()).map_err(ContractError::ConvertToBinary)
    })
}

pub fn supported_currency_pairs(storage: &dyn Storage) -> ContractResult<Binary> {
    SupportedPairs::load(storage)
        .map(|supported_pairs| supported_pairs.swap_pairs_df().collect())
        .and_then(|swap_pairs: Vec<_>| {
            to_json_binary(&swap_pairs).map_err(ContractError::ConvertToBinary)
        })
}

pub fn currencies(storage: &dyn Storage) -> ContractResult<Binary> {
    SupportedPairs::load(storage)
        .map(|supported_pairs| supported_pairs.currencies().collect())
        .and_then(|currencies: Vec<_>| {
            to_json_binary(&currencies).map_err(ContractError::ConvertToBinary)
        })
}

pub fn swap_path(
    storage: &dyn Storage,
    from: &SymbolSlice,
    to: &SymbolSlice,
) -> ContractResult<Binary> {
    SupportedPairs::load(storage)
        .and_then(|supported_pairs| supported_pairs.load_swap_path(from, to))
        .and_then(|swap_path| to_json_binary(&swap_path).map_err(ContractError::ConvertToBinary))
}

pub fn alarms_status(storage: &dyn Storage, now: Timestamp) -> ContractResult<Binary> {
    Oracle::<'_, _, PriceCurrencies>::load(storage)
        .and_then(|oracle| oracle.try_query_alarms(now))
        .and_then(|response| to_json_binary(&response).map_err(ContractError::ConvertToBinary))
}
