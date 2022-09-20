#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Storage,
    Timestamp,
};
use cw2::set_contract_version;

use finance::{currency::Nls, duration::Duration};
use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::batch::{Batch, Emit, Emitter};

use crate::{
    cmd::Dispatch,
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::Config,
    state::DispatchLog,
};

// version info for migration info
const CONTRACT_NAME: &str = env!("CARGO_PKG_NAME");
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let lpp_addr = validate_addr(deps.as_ref(), msg.lpp)?;
    let oracle_addr = validate_addr(deps.as_ref(), msg.oracle)?;
    let timealarms_addr = validate_addr(deps.as_ref(), msg.timealarms)?;
    let treasury_addr = validate_addr(deps.as_ref(), msg.treasury)?;

    Config::new(
        info.sender,
        msg.cadence_hours,
        lpp_addr,
        oracle_addr,
        timealarms_addr.clone(),
        treasury_addr,
        msg.tvl_to_apr,
    )
    .store(deps.storage)?;
    DispatchLog::update(deps.storage, env.block.time)?;

    let mut batch = Batch::default();
    batch
        .schedule_execute_wasm_no_reply::<_, Nls>(
            &timealarms_addr,
            &timealarms::msg::ExecuteMsg::AddAlarm {
                time: env.block.time + Duration::from_hours(msg.cadence_hours),
            },
            None,
        )
        .map_err(ContractError::from)?;

    Ok(Response::from(batch))
}

fn validate_addr(deps: Deps, addr: Addr) -> Result<Addr, ContractError> {
    deps.api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidContractAddress(addr))
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
        ExecuteMsg::TimeAlarm { time } => try_dispatch(deps, env, info, time),
    }
}

pub fn try_config(
    deps: DepsMut,
    info: MessageInfo,
    cadence_hours: u16,
) -> Result<Response, ContractError> {
    let config = Config::load(deps.storage)?;
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }
    Config::update(deps.storage, cadence_hours)?;

    Ok(Response::new().add_attribute("method", "config"))
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

pub fn try_dispatch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    time: Timestamp,
) -> Result<Response, ContractError> {
    let block_time = env.block.time;
    ensure!(time >= block_time, ContractError::AlarmTimeValidation {});
    let config = Config::load(deps.storage)?;

    if info.sender != config.timealarms {
        return Err(ContractError::UnrecognisedAlarm(info.sender));
    }
    let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
    let oracle = OracleRef::try_from(config.oracle.clone(), &deps.querier)?;

    let lpp_address = config.lpp.clone();
    let lpp = LppRef::try_new(lpp_address.clone(), &deps.querier)?;
    let emitter: Emitter = lpp.execute(
        Dispatch::new(oracle, last_dispatch, config, block_time, deps.querier)?,
        &deps.querier,
    )?;
    // Store the current time for use for the next calculation.
    DispatchLog::update(deps.storage, env.block.time)?;

    Ok(emitter
        .emit_tx_info(&env)
        .emit_to_string_value("to", lpp_address)
        .into())
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies_with_balance, mock_env, mock_info},
        Addr, DepsMut,
    };

    use crate::{
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
        state::tvl_intervals::{Intervals, Stop},
        ContractError,
    };

    use super::{execute, instantiate, query};

    fn do_instantiate(deps: DepsMut) {
        let msg = InstantiateMsg {
            cadence_hours: 10,
            lpp: Addr::unchecked("lpp"),
            oracle: Addr::unchecked("oracle"),
            timealarms: Addr::unchecked("timealarms"),
            treasury: Addr::unchecked("treasury"),
            tvl_to_apr: Intervals::from(vec![Stop::new(0, 5), Stop::new(1000000, 10)]).unwrap(),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res = instantiate(deps, mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        do_instantiate(deps.as_mut());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.cadence_hours);
    }

    #[test]
    fn configure() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));
        do_instantiate(deps.as_mut());

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

        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Config { cadence_hours: 20 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(20, value.cadence_hours);
    }
}
