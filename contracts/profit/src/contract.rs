use access_control::SingleUserAccess;
use finance::duration::Duration;
use platform::{
    message::Response as MessageResponse,
    response::{self},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo},
};
use versioning::{version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    profit::Profit,
    result::ContractResult,
    state::config::Config,
    ContractError,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        msg.timealarms.clone(),
    )
    .store(deps.storage)?;

    Config::new(msg.cadence_hours, msg.treasury).store(deps.storage)?;

    Profit::setup_alarm(
        msg.timealarms,
        &deps.querier,
        env.block.time,
        Duration::from_hours(msg.cadence_hours),
    )
    .map(response::response_only_messages)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .map_err(Into::into)
        .and_then(|label| response::response(&label))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            let alarm_recepient = env.contract.address.clone();
            try_transfer(deps.as_ref(), env, info).and_then(|resp| {
                response::response_with_messages::<_, _, ContractError>(&alarm_recepient, resp)
            })
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::Config { cadence_hours } => {
            Profit::try_config(deps.storage, cadence_hours).map(|()| response::empty_response())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Profit::query_config(deps.storage)?),
    }
    .map_err(Into::into)
}

fn try_transfer(deps: Deps<'_>, env: Env, info: MessageInfo) -> ContractResult<MessageResponse> {
    SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
        .check_access(&info.sender)?;

    Profit::transfer(deps, &env, info.sender)
}

#[cfg(test)]
mod tests {
    use currency::native::Nls;
    use finance::{currency::Currency, duration::Duration};
    use sdk::{
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{
            coins, from_binary,
            testing::{mock_dependencies_with_balance, mock_env, mock_info},
            to_binary, Addr, BankMsg, CosmosMsg, SubMsg, WasmMsg,
        },
        testing::customized_mock_deps_with_contracts,
    };

    use crate::{
        contract::sudo,
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, SudoMsg},
    };

    use super::{execute, instantiate, query};

    const TREASURY_ADDR: &str = "treasury";
    const TIMEALARMS_ADDR: &str = "timealarms";

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            cadence_hours: 10,
            treasury: Addr::unchecked(TREASURY_ADDR),
            timealarms: Addr::unchecked(TIMEALARMS_ADDR),
        }
    }
    #[test]
    fn proper_initialization() {
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(2, "token")),
            [TREASURY_ADDR, TIMEALARMS_ADDR],
        );

        let timealarms_addr = Addr::unchecked(TIMEALARMS_ADDR);
        let msg = InstantiateMsg {
            cadence_hours: 16,
            treasury: Addr::unchecked(TREASURY_ADDR),
            timealarms: timealarms_addr.clone(),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res: CwResponse = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
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
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(2, "token")),
            [TREASURY_ADDR, TIMEALARMS_ADDR],
        );

        let msg = instantiate_msg();
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let CwResponse {
            messages,
            attributes,
            events,
            data,
            ..
        }: CwResponse = sudo(
            deps.as_mut(),
            mock_env(),
            SudoMsg::Config { cadence_hours: 12 },
        )
        .unwrap();

        assert_eq!(messages.len(), 0);
        assert_eq!(attributes.len(), 0);
        assert_eq!(events.len(), 0);
        assert_eq!(data, None);

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(12, value.cadence_hours);
    }

    #[test]
    fn transfer() {
        use timealarms::msg::ExecuteMsg as AlarmsExecuteMsg;
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(20, Nls::BANK_SYMBOL)),
            [TREASURY_ADDR, TIMEALARMS_ADDR],
        );

        let msg = instantiate_msg();
        let info = mock_info("timealarms", &coins(2, "unolus"));
        let _res = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::TimeAlarm {};
        let res: CwResponse = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

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
