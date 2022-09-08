use cosmwasm_std::{Deps, DepsMut, MessageInfo, Response, Storage};
use finance::currency::SymbolOwned;

use crate::{msg::ConfigResponse, state::config::Config, ContractError};

pub fn query_config(deps: Deps) -> Result<ConfigResponse, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period_secs: config.price_feed_period_secs,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

pub fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period_secs: u32,
    feeders_percentage_needed: u8,
) -> Result<Response, ContractError> {
    Config::update(
        deps.storage,
        price_feed_period_secs,
        feeders_percentage_needed,
        info.sender,
    )?;

    Ok(Response::new())
}

pub fn try_configure_supported_pairs(
    storage: &mut dyn Storage,
    info: MessageInfo,
    pairs: Vec<(SymbolOwned, SymbolOwned)>,
) -> Result<Response, ContractError> {
    Config::update_supported_pairs(storage, pairs, info.sender)?;

    Ok(Response::new())
}
