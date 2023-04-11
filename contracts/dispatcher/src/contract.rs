use access_control::SingleUserAccess;
use currency::native::Nls;
use finance::duration::Duration;
use lpp::stub::LppRef;
use oracle::stub::OracleRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    response::{self},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Storage},
};
use versioning::{version, VersionSegment};

use crate::{
    cmd::Dispatch,
    msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, RewardScaleResponse,
        SudoMsg,
    },
    result::ContractResult,
    state::{Config, DispatchLog},
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
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    platform::contract::validate_addr(&deps.querier, &msg.lpp)?;
    platform::contract::validate_addr(&deps.querier, &msg.oracle)?;
    platform::contract::validate_addr(&deps.querier, &msg.timealarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.treasury)?;

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

    batch.schedule_execute_wasm_no_reply::<_, Nls>(
        &msg.timealarms,
        &timealarms::msg::ExecuteMsg::AddAlarm {
            time: env.block.time + Duration::from_hours(msg.cadence_hours),
        },
        None,
    )?;

    Ok(response::response_only_messages(batch))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    SingleUserAccess::remove_contract_owner(deps.storage);

    response::response(versioning::release())
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
            let resp = env.contract.address.clone();
            try_dispatch(deps, env, info).and_then(|messages| {
                response::response_with_messages::<_, _, ContractError>(&resp, messages)
            })
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::Config { cadence_hours } => {
            Config::update(deps.storage, cadence_hours).map(|()| response::empty_response())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps.storage)?),
        QueryMsg::RewardScale {} => to_binary(&query_reward_scale(deps.storage)?),
    }
    .map_err(Into::into)
}

fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
    Config::load(storage).map(|Config { cadence_hours, .. }| ConfigResponse { cadence_hours })
}

fn query_reward_scale(storage: &dyn Storage) -> StdResult<RewardScaleResponse> {
    Config::load(storage).map(|Config { tvl_to_apr, .. }| tvl_to_apr)
}

fn try_dispatch(deps: DepsMut<'_>, env: Env, info: MessageInfo) -> ContractResult<MessageResponse> {
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
    Ok(MessageResponse::messages_with_events(result.batch, emitter))
}

#[cfg(test)]
mod tests {
    use finance::percent::Percent;
    use sdk::{
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{
            coins, from_binary,
            testing::{mock_dependencies_with_balance, mock_env, mock_info},
            Addr, DepsMut,
        },
        testing::customized_mock_deps_with_contracts,
    };

    use crate::{
        contract::sudo,
        msg::{ConfigResponse, InstantiateMsg, QueryMsg, SudoMsg},
        state::reward_scale::{Bar, RewardScale, TotalValueLocked},
    };

    use super::{instantiate, query};

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

        let res: CwResponse = instantiate(deps, mock_env(), info, msg).unwrap();
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
        assert_eq!(value.cadence_hours, 12);

        let CwResponse {
            messages,
            attributes,
            events,
            data,
            ..
        }: CwResponse = sudo(
            deps.as_mut(),
            mock_env(),
            SudoMsg::Config { cadence_hours: 20 },
        )
        .unwrap();

        assert_eq!(messages.len(), 0);
        assert_eq!(attributes.len(), 0);
        assert_eq!(events.len(), 0);
        assert_eq!(data, None);

        // should now be 12
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(value.cadence_hours, 20);
    }
}
