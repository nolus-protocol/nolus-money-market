use finance::duration::Duration;
use platform::access_control::SingleUserAccess;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        ensure, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Timestamp,
    },
    cw2::set_contract_version,
};

use crate::{
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    profit::Profit,
    state::config::Config,
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

    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;

    SingleUserAccess::new(crate::access_control::OWNER_NAMESPACE, info.sender)
        .store(deps.storage)?;
    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        msg.timealarms.clone(),
    )
    .store(deps.storage)?;

    Config::new(msg.cadence_hours, msg.treasury).store(deps.storage)?;
    let subscribe_msg = Profit::alarm_subscribe_msg(
        &msg.timealarms,
        env.block.time,
        Duration::from_hours(msg.cadence_hours),
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_message(subscribe_msg))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config { cadence_hours } => {
            Profit::try_config(deps.storage, info, cadence_hours)
        }
        ExecuteMsg::TimeAlarm(time) => try_transfer(deps.as_ref(), env, info, time),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Profit::query_config(deps.storage)?),
    }
}

fn try_transfer(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    time: Timestamp,
) -> Result<Response, ContractError> {
    SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?.check_access(&info.sender)?;

    ensure!(
        time >= env.block.time,
        ContractError::AlarmTimeValidation {}
    );

    Ok(Profit::transfer(deps, env, info)?.into())
}

#[cfg(test)]
mod tests {
    use currency::native::Nls;
    use finance::{currency::Currency, duration::Duration};
    use sdk::cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies_with_balance, mock_env, mock_info},
        to_binary, Addr, BankMsg, CosmosMsg, SubMsg, WasmMsg,
    };

    use crate::{
        error::ContractError,
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    };

    use super::{execute, instantiate, query};

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            cadence_hours: 10,
            treasury: Addr::unchecked("treasury"),
            timealarms: Addr::unchecked("timealarms"),
        }
    }
    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        let timealarms_addr = Addr::unchecked("timealarms");
        let msg = InstantiateMsg {
            cadence_hours: 16,
            treasury: Addr::unchecked("treasury"),
            timealarms: timealarms_addr.clone(),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(1, res.messages.len());

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                funds: vec![],
                contract_addr: timealarms_addr.to_string(),
                msg: to_binary(&timealarms::msg::ExecuteMsg::AddAlarm {
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
    }

    #[test]
    fn transfer() {
        use timealarms::msg::ExecuteMsg as AlarmsExecuteMsg;
        let mut deps = mock_dependencies_with_balance(&coins(20, Nls::BANK_SYMBOL));

        let msg = instantiate_msg();
        let info = mock_info("timealarms", &coins(2, "unolus"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::TimeAlarm(mock_env().block.time);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(2, res.messages.len());
        println!("{:?}", res.messages);
        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(BankMsg::Send {
                    to_address: "treasury".to_string(),
                    amount: coins(20, Nls::BANK_SYMBOL),
                }),
                SubMsg::new(WasmMsg::Execute {
                    contract_addr: "timealarms".to_string(),
                    msg: to_binary(&AlarmsExecuteMsg::AddAlarm {
                        time: mock_env().block.time + Duration::from_hours(10)
                    })
                    .unwrap(),
                    funds: vec![]
                })
            ]
        );
    }
}
