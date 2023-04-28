use access_control::SingleUserAccess;
use dex::{Handler, Ics20Channel, Response as DexResponse, Result as DexResult};
use platform::{message::Response as MessageResponse, response};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo},
    neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
};
use versioning::{version, VersionSegment};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg},
    profit::Profit,
    result::ContractResult,
    state::{
        config::Config,
        contract_state::{ProfitMessageHandler as _, State, STATE},
    },
    ContractError,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;
    platform::contract::validate_addr(&deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;

    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;

    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        msg.timealarms.clone(),
    )
    .store(deps.storage)?;

    STATE.save(
        deps.storage,
        &State::new(
            &deps.querier,
            Config::new(msg.cadence_hours, msg.treasury),
            msg.connection_id,
            msg.oracle,
            msg.timealarms,
        )?,
    )?;

    Ok(response::empty_response())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software::<ContractError>(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .and_then(response::response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    mut deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
                .check_access(&info.sender)?;

            let alarm_recepient = env.contract.address.clone();

            try_time_alarm(deps.branch(), env).and_then(|resp| {
                response::response_with_messages::<_, _, ContractError>(&alarm_recepient, resp)
            })
        }
        ExecuteMsg::Config { cadence_hours } => {
            SingleUserAccess::check_owner_access::<ContractError>(deps.storage, &info.sender)?;

            let state: State = STATE.load(deps.storage)?;

            STATE.save(deps.storage, &state.try_update_config(cadence_hours)?)?;

            Ok(response::empty_response())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: NeutronSudoMsg) -> ContractResult<CwResponse> {
    let state: State = STATE.load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = match msg {
        NeutronSudoMsg::Response { data, .. } => {
            match state.on_response(data, deps.as_ref(), env) {
                DexResult::Continue(result) => result?,
                DexResult::Finished(response) => response,
            }
        }
        NeutronSudoMsg::Error { .. } => state.on_error(deps.as_ref(), env)?,
        NeutronSudoMsg::Timeout { .. } => state.on_timeout(deps.as_ref(), env)?,
        NeutronSudoMsg::OpenAck {
            channel_id,
            counterparty_channel_id,
            counterparty_version,
            ..
        } => state.confirm_open(
            deps.as_ref(),
            env,
            Ics20Channel {
                local_endpoint: channel_id,
                remote_endpoint: counterparty_channel_id,
            },
            counterparty_version,
        )?,
        NeutronSudoMsg::TxQueryResult { .. } => {
            unimplemented!()
        }
        NeutronSudoMsg::KVQueryResult { .. } => {
            unimplemented!()
        }
    };

    STATE.save(deps.storage, &next_state)?;

    Ok(response::response_only_messages(response))
}

fn try_time_alarm(deps: DepsMut<'_>, env: Env) -> ContractResult<MessageResponse> {
    let state: State = STATE.load(deps.storage)?;

    let DexResponse::<State> {
        response,
        next_state,
    } = match state.on_time_alarm(deps.as_ref(), env) {
        DexResult::Continue(result) => result?,
        DexResult::Finished(response) => response,
    };

    STATE.save(deps.storage, &next_state)?;

    Ok(response)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&Profit::query_config(deps.storage)?),
    }
    .map_err(Into::into)
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
            to_binary, Addr, BankMsg, SubMsg, WasmMsg,
        },
        neutron_sdk::sudo::msg::SudoMsg as NeutronSudoMsg,
        testing::customized_mock_deps_with_contracts,
    };

    use crate::{
        contract::sudo,
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    };

    use super::{execute, instantiate, query};

    const TREASURY_ADDR: &str = "treasury";
    const ORACLE_ADDR: &str = "oracle";
    const TIMEALARMS_ADDR: &str = "timealarms";

    fn instantiate_msg() -> InstantiateMsg {
        InstantiateMsg {
            cadence_hours: 10,
            treasury: Addr::unchecked(TREASURY_ADDR),
            oracle: Addr::unchecked(ORACLE_ADDR),
            timealarms: Addr::unchecked(TIMEALARMS_ADDR),
            connection_id: "dex-connection".into(),
        }
    }

    #[test]
    fn proper_initialization() {
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(2, "token")),
            [TREASURY_ADDR, ORACLE_ADDR, TIMEALARMS_ADDR],
        );

        let timealarms_addr = Addr::unchecked(TIMEALARMS_ADDR);
        let msg = InstantiateMsg {
            cadence_hours: 16,
            treasury: Addr::unchecked(TREASURY_ADDR),
            oracle: Addr::unchecked(ORACLE_ADDR),
            timealarms: timealarms_addr,
            connection_id: "dex-connection".into(),
        };
        let info = mock_info("creator", &coins(1000, "unolus"));

        let res: CwResponse = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res.messages.is_empty());

        let msg = NeutronSudoMsg::OpenAck {
            port_id: "transfer".to_string(),
            channel_id: "channel-1".to_string(),
            counterparty_channel_id: "channel-1".to_string(),
            counterparty_version: "1".to_string(),
        };
        let _res = sudo(deps.as_mut(), mock_env(), msg).unwrap();

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(16, value.cadence_hours);
    }

    // #[test]
    // fn configure() {
    //     let mut deps = customized_mock_deps_with_contracts(
    //         mock_dependencies_with_balance(&coins(2, "token")),
    //         [TREASURY_ADDR, TIMEALARMS_ADDR],
    //     );
    //
    //     let msg = instantiate_msg();
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //
    //     let CwResponse {
    //         messages,
    //         attributes,
    //         events,
    //         data,
    //         ..
    //     }: CwResponse = sudo(
    //         deps.as_mut(),
    //         mock_env(),
    //         SudoMsg::Config { cadence_hours: 12 },
    //     )
    //     .unwrap();
    //
    //     assert_eq!(messages.len(), 0);
    //     assert_eq!(attributes.len(), 0);
    //     assert_eq!(events.len(), 0);
    //     assert_eq!(data, None);
    //
    //     // should now be 12
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
    //     let value: ConfigResponse = from_binary(&res).unwrap();
    //     assert_eq!(12, value.cadence_hours);
    // }

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
