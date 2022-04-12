use std::convert::{TryFrom, TryInto};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Addr, Reply, CosmosMsg, SubMsg, Storage, Timestamp, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Decimal256, StdError};
use cw2::set_contract_version;
use marketprice::feed::{DenomPair, Observation};
use marketprice::market_price::PriceQuery;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ExecuteAlarmMsg};
use crate::state::{Config, CONFIG, FEEDERS, MARKET_PRICE, TIME_ORACLE, TIME_ALARMS};
use time_oracle::{Alarm, Id};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Config {
        base_asset: msg.base_asset,
        owner: info.sender,
        price_feed_period: msg.price_feed_period,
        feeders_percentage_needed: msg.feeders_percentage_needed,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &state)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config {
            price_feed_period,
            feeders_percentage_needed,
        } => try_configure(deps, info, price_feed_period, feeders_percentage_needed),
        ExecuteMsg::RegisterFeeder { feeder_address } => {
            try_register_feeder(deps, info, feeder_address)
        }
        ExecuteMsg::FeedPrice { base, prices } => try_feed_prices(deps, env, info, base, prices),
        ExecuteMsg::AddAlarm { addr, time } => try_add_alarm(deps, addr, time),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Feeders {} => to_binary(&FEEDERS.get(deps)?),
        QueryMsg::IsFeeder { address } => to_binary(&FEEDERS.is_registered(deps, &address)?),
        QueryMsg::Price { base, quote } => {
            to_binary(&query_market_price(deps, env, (base, quote))?)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    if msg.result.is_err() {
        Ok(Response::new().add_attribute("alarm", "error"))
    } else {
		TIME_ALARMS.remove(deps.storage, msg.id)?;
        Ok(Response::new().add_attribute("alarm", "success"))
    }
}

// this is a helper function so Decimal works with u64 rather than Uint128
// also, we must *round up* here, as we need 8, not 7 feeders to reach 50% of 15 total
fn feeders_needed(weight: usize, percentage: u8) -> usize {
    let weight128 = u128::try_from(weight).expect("usize to u128 overflow");
    let res = weight128 * u128::from(percentage) / 100;
    res.try_into().expect("usize overflow")
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: state.base_asset,
        owner: state.owner,
    })
}

fn query_market_price(deps: Deps, env: Env, denom_pair: DenomPair) -> StdResult<Observation> {
    let config = CONFIG.load(deps.storage)?;
    let price_feed_period = config.price_feed_period;

    let registered_feeders = FEEDERS.get(deps)?;
    let all_feeders_cnt = registered_feeders.len();
    let feeders_needed = feeders_needed(all_feeders_cnt, config.feeders_percentage_needed);

    let price_query = PriceQuery::new(denom_pair, price_feed_period, feeders_needed);

    let resp = MARKET_PRICE.get(deps.storage, env.block.time, price_query);
    match resp {
        Ok(feed) => Ok(feed),
        Err(err) => Err(StdError::generic_err(err.to_string())),
    }
}

fn try_configure(
    deps: DepsMut,
    info: MessageInfo,
    price_feed_period: u64,
    feeders_percentage_needed: u8,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    CONFIG.update(deps.storage, |mut c| -> StdResult<_> {
        c.price_feed_period = price_feed_period;
        c.feeders_percentage_needed = feeders_percentage_needed;
        Ok(c)
    })?;

    Ok(Response::new().add_attribute("method", "try_configure"))
}

fn try_register_feeder(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    // check if address is valid
    let f_address = deps.api.addr_validate(&address)?;
    FEEDERS.register(deps, f_address)?;

    Ok(Response::new().add_attribute("method", "try_register_feeder"))
}

fn try_feed_prices(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    base: String,
    prices: Vec<(String, Decimal256)>,
) -> Result<Response, ContractError> {
    let sender_raw = deps.api.addr_validate(info.sender.as_str())?;

    // Check feeder permission
    let is_registered = FEEDERS.is_registered(deps.as_ref(), &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }

    let config = CONFIG.load(deps.storage)?;
    let block_time = env.block.time;
    MARKET_PRICE.feed(
        deps.storage,
        block_time,
        sender_raw,
        base,
        prices,
        config.price_feed_period,
    )?;

    TIME_ORACLE.update_global_time(deps.storage, block_time)?;
    let response = try_notify_alarms(deps.storage, block_time)?;

    Ok(response.add_attribute("method", "try_feed_prices"))
}

fn try_add_alarm(deps: DepsMut, addr: Addr, time: Timestamp) -> Result<Response, ContractError> {
    let valid = deps
        .api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidAlarmAddess(addr))?;
    TIME_ALARMS.add(deps.storage, valid, time)?;
    Ok(Response::new().add_attribute("method", "try_add_alarm"))
}

fn try_notify_alarms(storage: &mut dyn Storage, ctime: Timestamp) -> StdResult<Response> {
    use time_oracle::AlarmDispatcher;

    struct OracleAlarmDispatcher<'a> {
        pub response: &'a mut Response,
    }

    impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
        fn send_to(&mut self, id: Id, alarm: Alarm, ctime: Timestamp) -> StdResult<()> {
            let msg = ExecuteAlarmMsg::Alarm(ctime);
            let wasm_msg = cosmwasm_std::wasm_execute(alarm.addr, &msg, vec![])?;
            let submsg = SubMsg::reply_always(CosmosMsg::Wasm(wasm_msg), id);
            self.response.messages.push(submsg);
            Ok(())
        }
    }

    let mut response = Response::new();
    let mut dispatcher = OracleAlarmDispatcher {
        response: &mut response,
    };

    TIME_ALARMS.notify(storage, &mut dispatcher, ctime)?;

    Ok(response)
}

#[test]
// we ensure this rounds up (as it calculates needed votes)
fn feeders_needed_rounds_properly() {
    // round up right below 1
    assert_eq!(7, feeders_needed(3, 255));
    // round up right over 1
    assert_eq!(7, feeders_needed(3, 254));
    assert_eq!(76, feeders_needed(30, 254));

    // exact matches don't round
    assert_eq!(17, feeders_needed(34, 50));
    assert_eq!(12, feeders_needed(48, 25));
}
