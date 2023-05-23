use access_control::SingleUserAccess;
use finance::{duration::Duration, percent::Percent};
use lpp::stub::LppRef;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
    response::{self},
};
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, StdResult,
        Storage, Timestamp,
    },
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{version, VersionSegment};

use crate::{
    cmd::{Dispatch, RewardCalculator},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
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

    setup_alarm(
        msg.timealarms,
        env.block.time,
        Duration::from_hours(msg.cadence_hours),
        &deps.querier,
    )
    .map(response::response_only_messages)
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))
        .and_then(response::response)
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
            SingleUserAccess::load(deps.storage, crate::access_control::TIMEALARMS_NAMESPACE)?
                .check_access(&info.sender)?;

            let resp = env.contract.address.clone();
            try_dispatch(deps, env, info.sender).and_then(|messages| {
                response::response_with_messages::<_, _, ContractError>(&resp, messages)
            })
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::Config { cadence_hours } => {
            Config::update_cadence_hours(deps.storage, cadence_hours)
                .map(|()| response::empty_response())
        }
        SudoMsg::Rewards { tvl_to_apr } => {
            Config::update_tvl_to_apr(deps.storage, tvl_to_apr).map(|()| response::empty_response())
        }
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn query(deps: Deps<'_>, _env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps.storage)?),
        QueryMsg::CalculateRewards {} => {
            to_binary(&query_reward(deps.storage, &deps.querier)?.units())
        }
    }
    .map_err(Into::into)
}

fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
    Config::load(storage).map(|Config { cadence_hours, .. }| ConfigResponse { cadence_hours })
}

fn query_reward(storage: &dyn Storage, querier: &QuerierWrapper<'_>) -> ContractResult<Percent> {
    let config: Config = Config::load(storage)?;

    LppRef::try_new(config.lpp, querier)?
        .execute(RewardCalculator::new(&config.tvl_to_apr), querier)
}

fn try_dispatch(deps: DepsMut<'_>, env: Env, timealarm: Addr) -> ContractResult<MessageResponse> {
    let now = env.block.time;

    let config = Config::load(deps.storage)?;
    let setup_alarm = setup_alarm(
        timealarm,
        now,
        Duration::from_hours(config.cadence_hours),
        &deps.querier,
    )?;

    let last_dispatch = DispatchLog::last_dispatch(deps.storage)?;

    let lpp_address = config.lpp.clone();
    let lpp = LppRef::try_new(lpp_address.clone(), &deps.querier)?;
    let result = lpp.execute(
        Dispatch::new(last_dispatch, config, now, &deps.querier)?,
        &deps.querier,
    )?;

    DispatchLog::update(deps.storage, env.block.time)?;

    let emitter = Emitter::of_type("tr-rewards")
        .emit_tx_info(&env)
        .emit_to_string_value("to", lpp_address)
        .emit_coin_dto("rewards", &result.receipt.in_nls);
    Ok(MessageResponse::messages_with_events(
        result.batch.merge(setup_alarm),
        emitter,
    ))
}

fn setup_alarm(
    timealarm: Addr,
    now: Timestamp,
    alarm_in: Duration,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Batch> {
    TimeAlarmsRef::new(timealarm, querier)
        .map_err(Into::into)
        .and_then(|stub| stub.setup_alarm(now + alarm_in).map_err(Into::into))
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
