#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Api, Binary, CosmosMsg, Decimal256, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdError, StdResult, Storage, SubMsg, Timestamp,
};
use cw2::set_contract_version;
use marketprice::feed::{Denom, DenomPair, Prices};
use marketprice::market_price::PriceQuery;
use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteAlarmMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, CONFIG, FEEDERS, MARKET_PRICE, TIME_ALARMS, TIME_ORACLE};
use time_oracle::Id;

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
        supported_denom_pairs: msg.supported_denom_pairs,
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
        ExecuteMsg::SupportedDenomPairs { pairs } => {
            try_configure_supported_pairs(deps.storage, info, pairs)
        }
        ExecuteMsg::FeedPrices { prices } => try_feed_multiple_prices(
            deps.storage,
            env.block.time,
            get_sender(deps.api, info)?,
            prices,
        ),
        ExecuteMsg::AddAlarm { addr, time } => try_add_alarm(deps, addr, time),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Feeders {} => to_binary(&FEEDERS.get(deps.storage)?),
        QueryMsg::IsFeeder { address } => {
            to_binary(&FEEDERS.is_registered(deps.storage, &address)?)
        }
        QueryMsg::PriceFor { denom } => {
            to_binary(&query_market_price_for(deps.storage, env, denom)?)
        }
        QueryMsg::SupportedDenomPairs {} => {
            to_binary(&CONFIG.load(deps.storage)?.supported_denom_pairs)
        }
    }
}

pub fn get_sender(api: &dyn Api, info: MessageInfo) -> StdResult<Addr> {
    api.addr_validate(info.sender.as_str())
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
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        base_asset: config.base_asset,
        owner: config.owner,
        price_feed_period: config.price_feed_period,
        feeders_percentage_needed: config.feeders_percentage_needed,
    })
}

fn query_market_price_for(storage: &dyn Storage, env: Env, denom: Denom) -> StdResult<Decimal256> {
    let price_query = init_price_query(storage, denom, None)?;
    let resp = MARKET_PRICE.get(storage, env.block.time, price_query);
    match resp {
        Ok(feed) => Ok(feed),
        Err(err) => Err(StdError::generic_err(err.to_string())),
    }
}

fn assert_supported_denom(
    supported_denom_pairs: Vec<(String, String)>,
    denom: Denom,
) -> StdResult<()> {
    let mut all_supported_denoms = HashSet::<Denom>::new();
    for pair in supported_denom_pairs {
        all_supported_denoms.insert(pair.0);
        all_supported_denoms.insert(pair.1);
    }
    if !all_supported_denoms.contains(&denom) {
        return Err(StdError::generic_err("Unsupported denom"));
    }
    Ok(())
}

fn init_price_query(
    storage: &dyn Storage,
    base: Denom,
    quote: Option<Denom>,
) -> StdResult<PriceQuery> {
    let config = CONFIG.load(storage)?;
    let price_feed_period = config.price_feed_period;

    let query_quote = match quote {
        Some(q) => q,
        None => config.base_asset,
    };
    assert_supported_denom(config.supported_denom_pairs, base.clone())?;

    let registered_feeders = FEEDERS.get(storage)?;
    let all_feeders_cnt = registered_feeders.len();
    let feeders_needed = feeders_needed(all_feeders_cnt, config.feeders_percentage_needed);

    Ok(PriceQuery::new(
        (base, query_quote),
        price_feed_period,
        feeders_needed,
    ))
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

fn try_configure_supported_pairs(
    storage: &mut dyn Storage,
    info: MessageInfo,
    pairs: Vec<DenomPair>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    for pair in &pairs {
        if pair.0.eq_ignore_ascii_case(pair.1.as_str()) {
            return Err(ContractError::InvalidDenomPair(pair.to_owned()));
        }
    }

    CONFIG.update(storage, |mut c| -> StdResult<_> {
        c.supported_denom_pairs = pairs;
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
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    base: Denom,
    prices: Vec<(Denom, Decimal256)>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(storage)?;

    let filtered_prices = remove_invalid_prices(config.supported_denom_pairs, base.clone(), prices);
    if filtered_prices.is_empty() {
        return Err(ContractError::UnsupportedDenomPairs {});
    }

    MARKET_PRICE.feed(
        storage,
        block_time,
        sender_raw,
        base,
        filtered_prices,
        config.price_feed_period,
    )?;

    Ok(Response::default())
}

fn try_add_alarm(deps: DepsMut, addr: Addr, time: Timestamp) -> Result<Response, ContractError> {
    let valid = deps
        .api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidAlarmAddress(addr))?;
    TIME_ALARMS.add(deps.storage, valid, time)?;
    Ok(Response::new().add_attribute("method", "try_add_alarm"))
}

fn try_notify_alarms(storage: &mut dyn Storage, ctime: Timestamp) -> StdResult<Response> {
    use time_oracle::AlarmDispatcher;

    struct OracleAlarmDispatcher<'a> {
        pub response: &'a mut Response,
    }

    impl<'a> AlarmDispatcher for OracleAlarmDispatcher<'a> {
        fn send_to(&mut self, id: Id, addr: Addr, ctime: Timestamp) -> StdResult<()> {
            let msg = ExecuteAlarmMsg::Alarm(ctime);
            let wasm_msg = cosmwasm_std::wasm_execute(addr, &msg, vec![])?;
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

fn try_feed_multiple_prices(
    storage: &mut dyn Storage,
    block_time: Timestamp,
    sender_raw: Addr,
    prices: Vec<Prices>,
) -> Result<Response, ContractError> {
    // Check feeder permission
    let is_registered = FEEDERS.is_registered(storage, &sender_raw)?;
    if !is_registered {
        return Err(ContractError::UnknownFeeder {});
    }
    for entry in prices {
        try_feed_prices(
            storage,
            block_time,
            sender_raw.clone(),
            entry.base,
            entry.values,
        )?;
    }
    TIME_ORACLE.update_global_time(storage, block_time)?;
    let response = try_notify_alarms(storage, block_time)?;
    Ok(response.add_attribute("method", "try_feed_prices"))
}

fn remove_invalid_prices(
    supported_denom_pairs: Vec<(String, String)>,
    base: Denom,
    prices: Vec<(Denom, Decimal256)>,
) -> Vec<(String, Decimal256)> {
    prices
        .iter()
        .filter(|price| {
            supported_denom_pairs.contains(&(base.clone(), price.0.clone()))
                && !base.eq_ignore_ascii_case(price.0.as_str())
        })
        .map(|p| p.to_owned())
        .collect()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use cosmwasm_std::Decimal256;

    use crate::contract::{feeders_needed, remove_invalid_prices};

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

    #[test]
    fn test_remove_invalid_prices() {
        let supported_pairs = vec![
            ("A".to_string(), "B".to_string()),
            ("A".to_string(), "C".to_string()),
            ("B".to_string(), "A".to_string()),
            ("C".to_string(), "D".to_string()),
        ];

        let filtered = remove_invalid_prices(
            supported_pairs,
            "B".to_string(),
            vec![
                ("A".to_string(), Decimal256::from_str("1.2").unwrap()),
                ("D".to_string(), Decimal256::from_str("3.2").unwrap()),
                ("B".to_string(), Decimal256::from_str("1.2").unwrap()),
            ],
        );

        assert_eq!(
            vec![("A".to_string(), Decimal256::from_str("1.2").unwrap()),],
            filtered
        );
    }
}
