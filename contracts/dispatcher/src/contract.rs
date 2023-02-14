use access_control::SingleUserAccess;
use currency::native::Nls;
use finance::duration::Duration;
use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::batch::{Batch, Emit, Emitter};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    cmd::Dispatch,
    error::ContractError,
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    state::Config,
    state::DispatchLog,
};

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    platform::contract::validate_addr(&deps.querier, &msg.lpp)?;
    platform::contract::validate_addr(&deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;

    SingleUserAccess::new_contract_owner(info.sender).store(deps.storage)?;
    SingleUserAccess::new(
        crate::access_control::TIMEALARMS_NAMESPACE,
        msg.timealarms.clone(),
    )
    .store(deps.storage)?;

    Config::new(
        msg.cadence_hours,
        msg.lpp,
        msg.oracle,
        msg.treasury,
        msg.tvl_to_apr,
    )
    .store(deps.storage)?;
    DispatchLog::update(deps.storage, env.block.time)?;

    let mut batch = Batch::default();

    batch
        .schedule_execute_wasm_no_reply::<_, Nls>(
            &msg.timealarms,
            &timealarms::msg::ExecuteMsg::AddAlarm {
                time: env.block.time + Duration::from_hours(msg.cadence_hours),
            },
            None,
        )
        .map_err(ContractError::from)?;

    Ok(Response::from(batch))
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MigrateMsg {}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Config { cadence_hours } => try_config(deps.storage, info, cadence_hours),
        ExecuteMsg::TimeAlarm {} => try_dispatch(deps, env, info),
    }
}

pub fn try_config(
    storage: &mut dyn Storage,
    info: MessageInfo,
    cadence_hours: u16,
) -> Result<Response, ContractError> {
    SingleUserAccess::check_owner_access::<ContractError>(storage, &info.sender)?;

    Config::update(storage, cadence_hours)?;

    Ok(Response::new().add_attribute("method", "config"))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps.storage)?),
    }
}

fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
    let Config { cadence_hours, .. } = Config::load(storage)?;

    Ok(ConfigResponse { cadence_hours })
}

pub fn try_dispatch(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let block_time = env.block.time;

    SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
        .check_access(&info.sender)?;

    let config = Config::load(deps.storage)?;

    let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;
    let oracle = OracleRef::try_from(config.oracle.clone(), &deps.querier)?;

    let lpp_address = config.lpp.clone();
    let lpp = LppRef::try_new(lpp_address.clone(), &deps.querier)?;
    let result = lpp.execute(
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

    let emitter = Emitter::of_type("tr-rewards")
        .emit_tx_info(&env)
        .emit_to_string_value("to", lpp_address)
        .emit_coin_dto("rewards", result.receipt.in_nls);

    Ok(result.batch.into_response(emitter))
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent;
    use sdk::cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies_with_balance, mock_env, mock_info},
        Addr, DepsMut,
    };
    use sdk::testing::customized_mock_deps_with_contracts;

    use crate::{
        msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
        state::reward_scale::{Bar, RewardScale, TotalValueLocked},
        ContractError,
    };

    use super::{execute, instantiate, query};

    const LPP_ADDR: &str = "lpp";
    const ORACLE_ADDR: &str = "oracle";
    const TIMEALARMS_ADDR: &str = "timealarms";
    const TREASURY_ADDR: &str = "treasury";

    fn do_instantiate(deps: DepsMut<'_>) {
        let msg = InstantiateMsg {
            cadence_hours: 10,
            lpp: Addr::unchecked(LPP_ADDR),
            oracle: Addr::unchecked(ORACLE_ADDR),
            timealarms: Addr::unchecked(TIMEALARMS_ADDR),
            treasury: Addr::unchecked(TREASURY_ADDR),
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
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(2, "token")),
            [LPP_ADDR, TIMEALARMS_ADDR, ORACLE_ADDR, TREASURY_ADDR],
        );
        do_instantiate(deps.as_mut());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(10, value.cadence_hours);
    }

    #[test]
    fn configure() {
        let mut deps = customized_mock_deps_with_contracts(
            mock_dependencies_with_balance(&coins(2, "token")),
            [LPP_ADDR, TIMEALARMS_ADDR, ORACLE_ADDR, TREASURY_ADDR],
        );

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
