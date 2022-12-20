use currency::native::Nls;
use finance::duration::Duration;
use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::{
    access_control::SingleUserAccess,
    batch::{Batch, Emit, Emitter},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        ensure, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Storage,
        Timestamp,
    },
    cw2::set_contract_version,
};

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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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

    SingleUserAccess::new(crate::access_control::OWNER_NAMESPACE, info.sender)
        .store(deps.storage)?;
    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        timealarms_addr.clone(),
    )
    .store(deps.storage)?;

    Config::new(
        msg.cadence_hours,
        lpp_addr,
        oracle_addr,
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

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config { cadence_hours } => try_config(deps.storage, info, cadence_hours),
        ExecuteMsg::TimeAlarm(time) => try_dispatch(deps, env, info, time),
    }
}

pub fn try_config(
    storage: &mut dyn Storage,
    info: MessageInfo,
    cadence_hours: u16,
) -> Result<Response, ContractError> {
    SingleUserAccess::load(storage, crate::access_control::OWNER_NAMESPACE)?
        .check_access(&info.sender)?;

    Config::update(storage, cadence_hours)?;

    Ok(Response::new().add_attribute("method", "config"))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
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

    SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
        .check_access(&info.sender)?;

    let config = Config::load(deps.storage)?;

    let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
    let oracle = OracleRef::try_from(config.oracle.clone(), &deps.querier)?;

    let lpp_address = config.lpp.clone();
    let lpp = LppRef::try_new(lpp_address.clone(), &deps.querier)?;
    let emitter: Emitter = lpp.execute(
        Dispatch::new(
            deps.storage,
            oracle,
            last_dispatch,
            config,
            block_time,
            deps.querier,
        )?,
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
    use finance::percent::Percent;
    use sdk::cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies_with_balance, mock_env, mock_info},
        Addr, DepsMut,
    };

    use crate::{
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
        state::reward_scale::{Bar, RewardScale, TotalValueLocked},
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
            tvl_to_apr: RewardScale::try_from(vec![
                Bar {
                    tvl: TotalValueLocked::new(0),
                    apr: Percent::from_permille(5),
                },
                Bar {
                    tvl: TotalValueLocked::new(1000),
                    apr: Percent::from_permille(10),
                },
            ])
            .unwrap(),
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
            Err(ContractError::Unauthorized(..)) => {}
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
