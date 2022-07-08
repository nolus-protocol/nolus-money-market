#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
    Storage, Timestamp,
};
use cw2::set_contract_version;
use marketprice::feed::{Denom, DenomPair, Prices};

use crate::alarms::MarketAlarms;
use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, PriceResponse, QueryMsg};
use crate::oracle::MarketOracle;
use crate::state::config::Config;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let timealarms_addr = deps.api.addr_validate(&msg.timealarms_addr)?;

    Config::new(
        msg.base_asset,
        info.sender,
        msg.price_feed_period_secs,
        msg.feeders_percentage_needed,
        msg.supported_denom_pairs,
        timealarms_addr,
    )
    .store(deps.storage)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config {
            price_feed_period_secs,
            feeders_percentage_needed,
        } => try_configure(
            deps,
            info,
            price_feed_period_secs,
            feeders_percentage_needed,
        ),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            try_register_feeder(deps, info, feeder_address)
        }
        ExecuteMsg::SupportedDenomPairs { pairs } => {
            try_configure_supported_pairs(deps.storage, info, pairs)
        }
        ExecuteMsg::FeedPrices { prices } => try_feed_multiple_prices(
            deps.storage,
            env.block.time,
            get_sender(deps.api, info)?,
            prices,
        ),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Feeders {} => to_binary(&MarketOracle::get_feeders(deps.storage)?),
        QueryMsg::IsFeeder { address } => {
            to_binary(&MarketOracle::is_feeder(deps.storage, &address)?)
        }
        QueryMsg::PriceFor { denoms } => {
            to_binary(&query_market_price_for(deps.storage, env, denoms)?)
        }
        QueryMsg::SupportedDenomPairs {} => {
            to_binary(&Config::load(deps.storage)?.supported_denom_pairs)
        }
    }
}

pub fn get_sender(api: &dyn Api, info: MessageInfo) -> StdResult<Addr> {
    api.addr_validate(info.sender.as_str())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match msg.result {
        cosmwasm_std::SubMsgResult::Ok(_) => {
            MarketAlarms::remove(deps.storage, msg.id)?;
            Response::new().add_attribute("alarm", "success")
        }
        cosmwasm_std::SubMsgResult::Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err),
    };
    Ok(resp)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = Config::load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period_secs: config.price_feed_period_secs,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

fn query_market_price_for(
    storage: &dyn Storage,
    env: Env,
    denoms: Vec<Denom>,
) -> StdResult<PriceResponse> {
    Ok(PriceResponse {
        prices: MarketOracle::get_price_for(storage, env.block.time, denoms)?,
    })
}

fn try_configure(
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

    Ok(Response::new().add_attribute("method", "try_configure"))
}

fn try_configure_supported_pairs(
    storage: &mut dyn Storage,
    info: MessageInfo,
    pairs: Vec<DenomPair>,
) -> Result<Response, ContractError> {
    Config::update_supported_pairs(storage, pairs, info.sender)?;

    Ok(Response::new().add_attribute("method", "try_configure_supported_pairs"))
}

fn try_register_feeder(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    // check if address is valid
    let f_address = deps.api.addr_validate(&address)?;
    MarketOracle::register_feeder(deps, f_address)?;

    Ok(Response::new().add_attribute("method", "try_register_feeder"))
}

fn try_feed_multiple_prices(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<Prices>,
) -> Result<Response, ContractError> {
    // Check feeder permission
    let is_registered = MarketOracle::is_feeder(storage, &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }
    for entry in prices {
        MarketOracle::feed_prices(storage, block_time, &sender_raw, entry.base, entry.values)?;
    }
    let submsg = MarketAlarms::trigger_time_alarms(storage)?;
    Ok(Response::new()
        .add_submessage(submsg)
        .add_attribute("method", "try_feed_prices"))
}
