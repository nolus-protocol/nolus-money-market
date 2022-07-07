#[cfg(feature = "cosmwasm-bindings")]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Timestamp,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::profit::Profit;
use crate::state::config::Config;

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

    let treasury = validate_addr(deps.as_ref(), msg.treasury)?;
    let oracle = validate_addr(deps.as_ref(), msg.oracle)?;

    Config::new(info.sender, msg.cadence_hours, treasury, oracle.clone()).store(deps.storage)?;
    let subscribe_msg = Profit::alarm_subscribe_msg(&oracle, env.block.time, msg.cadence_hours)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_message(subscribe_msg))
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config { cadence_hours } => Profit::try_config(deps, info, cadence_hours),
        ExecuteMsg::Alarm { time } => try_transfer(deps, env, info, time),
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Profit::query_config(deps.storage)?),
    }
}

fn validate_addr(deps: Deps, addr: Addr) -> Result<Addr, ContractError> {
    deps.api
        .addr_validate(addr.as_str())
        .map_err(|_| ContractError::InvalidContractAddress(addr))
}

fn try_transfer(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    time: Timestamp,
) -> Result<Response, ContractError> {
    ensure!(
        time >= env.block.time,
        ContractError::AlarmTimeValidation {}
    );
    Profit::transfer(deps, env, info)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies_with_balance, mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, SubMsg, WasmMsg,
    };

    use finance::duration::Duration;

    use super::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            cadence_hours: 10,
            treasury: Addr::unchecked("treasury"),
            oracle: Addr::unchecked("time"),
        }
    }
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let oracle_addr = Addr::unchecked("time");
        let msg = InstantiateMsg {
            cadence_hours: 16,
            treasury: Addr::unchecked("treasury"),
            oracle: oracle_addr.clone(),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                funds: vec![],
                contract_addr: oracle_addr.to_string(),
                msg: to_binary(&oracle::msg::ExecuteMsg::AddAlarm {
                    time: mock_env().block.time + Duration::from_hours(16),
                })
                .unwrap(),
            }))]
        );

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(16, value.cadence_hours);
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

    #[test]
    fn transfer() {
        let mut deps = mock_dependencies_with_balance(&coins(20, "unolus"));

        let msg = instantiate_msg();
        let info = mock_info("time", &coins(2, "unolus"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::Alarm {
            time: mock_env().block.time,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(1, res.messages.len());
        println!("{:?}", res.messages);
        assert_eq!(
            res.messages,
            vec![SubMsg::new(BankMsg::Send {
                to_address: "treasury".to_string(),
                amount: coins(20, "unolus"),
            })]
        );
    }
}
