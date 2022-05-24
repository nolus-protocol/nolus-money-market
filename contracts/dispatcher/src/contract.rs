#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Storage, SubMsg, Timestamp, WasmMsg,
};
use cw2::set_contract_version;
use finance::interest::InterestPeriod;
use time_oracle::Alarms;

use crate::dispatcher::{exec_lpp_distribute_rewards, get_lpp_balance, swap_reward_in_unls};
use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::config::Config;
use crate::state::dispatch_log::DispatchLog;

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const TIME_ALARMS: Alarms = Alarms::new("alarms", "alarms_idx", "alarms_next_id");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lpp = validate_addr(deps.as_ref(), msg.lpp)?;
    let time_oracle = validate_addr(deps.as_ref(), msg.time_oracle)?;
    let market_oracle = validate_addr(deps.as_ref(), msg.market_oracle)?;

    Config::new(
        info.sender,
        msg.cadence_hours,
        lpp,
        time_oracle,
        msg.treasury,
        market_oracle,
        msg.tvl_to_apr,
    )
    .store(deps.storage)?;

    try_add_alarm(
        deps,
        env.contract.address,
        env.block.time.plus_seconds(to_seconds(msg.cadence_hours)),
    )?;

    Ok(Response::new().add_attribute("method", "instantiate"))
}

fn validate_addr(deps: Deps, addr: Addr) -> Result<Addr, ContractError> {
    deps.api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidContractAddress(addr))
}
fn to_seconds(cadence_hours: u32) -> u64 {
    cadence_hours as u64 * 60 * 60
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config { cadence_hours } => try_config(deps, info, cadence_hours),
        ExecuteMsg::Alarm { time } => try_dispatch(deps, env, info, time),
    }
}

pub fn try_config(
    deps: DepsMut,
    info: MessageInfo,
    cadence_hours: u32,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    Config::update(deps.storage, cadence_hours)?;

    Ok(Response::new().add_attribute("method", "config"))
}

pub fn try_dispatch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    _time: Timestamp,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;

    if info.sender != config.time_oracle {
        return Err(ContractError::UnrecognisedAlarm(info.sender));
    }

    // 1. get LPP balance: TVL = BalanceLPN + TotalPrincipalDueLPN + TotalInterestDueLPN
    let lpp_balance = get_lpp_balance(deps.as_ref(), config.lpp.clone())?;

    // 2. get apr from configuration
    let arp_permille = config.tvl_to_apr.get_apr(lpp_balance.amount.u128())?;

    // 3. Use the finance::interest::interestPeriod::interest() to calculate the reward in LPN,
    //    which matches TVLdenom, since the last calculation, Rewards_TVLdenom
    let reward_lppdenom = InterestPeriod::with_interest(arp_permille)
        .from(DispatchLog::last_dispatch(deps.storage)?)
        .interest(&lpp_balance);
    // 4. Store the current time for use for the next calculation.
    DispatchLog::update(deps.storage, env.block.time)?;

    // 5. Obtain the currency market price of TVLdenom in uNLS and convert Rewards_TVLdenom in uNLS, Rewards_uNLS.
    let _reward_unls = swap_reward_in_unls(deps.as_ref(), config.market_oracle, reward_lppdenom)?;

    // 6. Prepare a Send Rewards for the amount of Rewards_uNLS to the Treasury.
    // 7. LPP.Distribute Rewards command.
    Ok(
        Response::new().add_submessages(vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            funds: vec![],
            contract_addr: config.lpp.to_string(),
            msg: to_binary(&exec_lpp_distribute_rewards())?,
        }))]),
    )
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps.storage)?),
    }
}

fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
    let config = Config::load(storage)?;
    Ok(ConfigResponse {
        cadence_hours: config.cadence_hours,
    })
}

fn try_add_alarm(deps: DepsMut, addr: Addr, time: Timestamp) -> Result<Response, ContractError> {
    let valid = deps
        .api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidAlarmAddress(addr))?;
    TIME_ALARMS.add(deps.storage, valid, time)?;
    Ok(Response::new().add_attribute("method", "try_add_alarm"))
}

#[cfg(test)]
mod tests {
    use crate::msg::ConfigResponse;
    use crate::state::tvl_intervals::{Intervals, Stop};

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies_with_balance, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, Addr};

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            cadence_hours: 10,
            lpp: Addr::unchecked("lpp"),
            time_oracle: Addr::unchecked("time"),
            treasury: Addr::unchecked("treasury"),
            market_oracle: Addr::unchecked("market_oracle"),
            tvl_to_apr: Intervals::from(vec![Stop::new(0, 5), Stop::new(1000000, 10)]).unwrap(),
        }
    }
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = instantiate_msg();
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.cadence_hours);
    }

    #[test]
    fn configure() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let msg = instantiate_msg();
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Config { cadence_hours: 20 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Config { cadence_hours: 12 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(12, value.cadence_hours);
    }

    // #[test]
    // fn transfer() {
    //     let mut deps = mock_dependencies_with_balance(&coins(20, "unolus"));

    //     let msg = instantiate_msg();
    //     let info = mock_info("time", &coins(2, "unolus"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

    //     let msg = ExecuteMsg::Alarm {
    //         time: mock_env().block.time,
    //     };
    //     let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     assert_eq!(1, res.messages.len());
    //     println!("{:?}", res.messages);
    //     assert_eq!(
    //         res.messages,
    //         vec![SubMsg::new(BankMsg::Send {
    //             to_address: "treasury".to_string(),
    //             amount: coins(20, "unolus"),
    //         })]
    //     );
    // }
}
