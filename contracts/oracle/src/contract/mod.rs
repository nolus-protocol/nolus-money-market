use std::collections::{HashMap, HashSet};

#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    Storage, Timestamp,
};
use cw2::set_contract_version;

use crate::{
    contract_validation::validate_contract_addr,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    state::config::Config,
};
use finance::{
    currency::{Nls, SymbolOwned},
    price::PriceDTO,
};

use self::{
    alarms::MarketAlarms,
    config::{query_config, try_configure, try_configure_supported_pairs},
    feed::Feeds,
    feeder::Feeders,
    query::{query_market_price_for, QueryWithOracleBase},
};

mod alarms;
mod config;
mod feed;
mod feeder;
pub mod query;

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
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Config::new(
        msg.base_asset,
        info.sender,
        msg.price_feed_period_secs,
        msg.feeders_percentage_needed,
        msg.supported_denom_pairs,
        deps.api.addr_validate(&msg.timealarms_addr)?,
    )
    .store(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::Config {} => Ok(to_binary(&query_config(deps)?)?),
        QueryMsg::Feeders {} => Ok(to_binary(&Feeders::get(deps.storage)?)?),
        QueryMsg::IsFeeder { address } => {
            Ok(to_binary(&Feeders::is_feeder(deps.storage, &address)?)?)
        }
        QueryMsg::SupportedDenomPairs {} => Ok(to_binary(
            &Config::load(deps.storage)?.supported_denom_pairs,
        )?),
        _ => Ok(QueryWithOracleBase::cmd(deps, env, msg)?),
    }
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
            Feeders::try_register(deps, info, feeder_address)
        }
        ExecuteMsg::SupportedDenomPairs { pairs } => {
            try_configure_supported_pairs(deps.storage, info, pairs)
        }
        ExecuteMsg::FeedPrices { prices } => {
            try_feed_prices(deps.storage, env.block.time, info.sender, prices)
        }
        ExecuteMsg::AddPriceAlarm { alarm } => {
            validate_contract_addr(&deps.querier, &info.sender)?;
            MarketAlarms::try_add_price_alarm(deps.storage, info.sender, alarm)
        }
        ExecuteMsg::RemovePriceAlarm {} => MarketAlarms::remove(deps.storage, info.sender),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let resp = match msg.result {
        cosmwasm_std::SubMsgResult::Ok(resp) => {
            let data = match resp.data {
                Some(d) => d,
                None => return Ok(err_as_ok("No data")),
            };
            MarketAlarms::remove(deps.storage, from_binary(&data)?)?;
            Response::new().add_attribute("alarm", "success")
        }
        cosmwasm_std::SubMsgResult::Err(err) => Response::new()
            .add_attribute("alarm", "error")
            .add_attribute("error", err),
    };
    Ok(resp)
}

fn err_as_ok(err: &str) -> Response {
    Response::new()
        .add_attribute("alarm", "error")
        .add_attribute("error", err)
}

fn try_feed_prices(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<PriceDTO>,
) -> Result<Response, ContractError> {
    // Check feeder permission
    let is_registered = Feeders::is_feeder(storage, &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }

    let config = Config::load(storage)?;
    let oracle = Feeds::with(config.clone());

    // Store the new price feed
    oracle.feed_prices(storage, block_time, &sender_raw, prices)?;

    // // Get all currencies registered for alarms
    // let hooks_currencies = MarketAlarms::get_hooks_currencies(storage)?;

    // //re-calculate the price of these currencies
    // let updated_prices: HashMap<SymbolOwned, PriceDTO> =
    //     oracle.get_prices(storage, block_time, hooks_currencies)?;

    // // try notify affected subscribers
    // let mut batch = MarketAlarms::try_notify_hooks(storage, updated_prices)?;
    // batch.schedule_execute_wasm_reply_error::<_, Nls>(
    //     &config.timealarms_contract,
    //     timealarms::msg::ExecuteMsg::Notify(),
    //     None,
    //     1,
    // )?;
    // Ok(Response::from(batch))
    Ok(Response::default())
}
