use std::ops::{Deref, DerefMut};

use access_control::SingleUserAccess;
use finance::{duration::Duration, percent::Percent, period::Period};
use lpp_platform::UsdGroup;
use platform::{batch::Batch, message::Response as MessageResponse, response};
#[cfg(feature = "cosmwasm-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, StdResult,
        Storage, Timestamp,
    },
};
use timealarms::stub::TimeAlarmsRef;
use versioning::{package_version, version, SemVer, Version, VersionSegment};

use crate::{
    cmd::{self, RewardCalculator},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, SudoMsg},
    result::ContractResult,
    state::{Config, DispatchLog},
};

const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 1;
const PACKAGE_VERSION: SemVer = package_version!();
const CONTRACT_VERSION: Version = version!(CONTRACT_STORAGE_VERSION, PACKAGE_VERSION);

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn instantiate(
    mut deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> ContractResult<CwResponse> {
    versioning::initialize(deps.storage, CONTRACT_VERSION)?;

    platform::contract::validate_addr(deps.querier, &msg.protocol.lpp)?;
    platform::contract::validate_addr(deps.querier, &msg.protocol.oracle)?;
    platform::contract::validate_addr(deps.querier, &msg.timealarms)?;
    platform::contract::validate_addr(deps.querier, &msg.treasury)?;

    SingleUserAccess::new(
        deps.storage.deref_mut(),
        crate::access_control::TIMEALARMS_NAMESPACE,
    )
    .grant_to(&msg.timealarms)?;

    Config::new(
        msg.cadence_hours,
        msg.protocol,
        msg.treasury,
        msg.tvl_to_apr,
    )
    .store(deps.storage)?;
    DispatchLog::update(deps.storage, env.block.time)?;

    setup_alarm(
        msg.timealarms,
        env.block.time,
        Duration::from_hours(msg.cadence_hours),
        deps.querier,
    )
    .map(response::response_only_messages)
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> ContractResult<CwResponse> {
    use crate::state::migration;
    versioning::update_software_and_storage::<CONTRACT_STORAGE_VERSION_FROM, _, _, _, _>(
        deps.storage,
        CONTRACT_VERSION,
        |storage: &mut dyn Storage| migration::migrate(storage),
        Into::into,
    )
    .map(|(label, ())| label)
    .and_then(response::response)
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    match msg {
        ExecuteMsg::TimeAlarm {} => {
            SingleUserAccess::new(
                deps.storage.deref(),
                crate::access_control::TIMEALARMS_NAMESPACE,
            )
            .check(&info.sender)?;

            try_dispatch(deps, &env, info.sender).map(response::response_only_messages)
        }
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, _env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    match msg {
        SudoMsg::Config { cadence_hours } => {
            Config::update_cadence_hours(deps.storage, cadence_hours)
                .map(|()| response::empty_response())
        }
        SudoMsg::Rewards { tvl_to_apr } => {
            Config::update_tvl_to_apr(deps.storage, tvl_to_apr).map(|()| response::empty_response())
        }
        SudoMsg::AddProtocol(protocol) => {
            Config::add_protocol(deps.storage, protocol).map(|()| response::empty_response())
        }
    }
}

#[cfg_attr(feature = "cosmwasm-bindings", entry_point)]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps.storage)?),
        QueryMsg::CalculateRewards {} => {
            to_json_binary(&query_reward(deps.storage, deps.querier, &env)?.units())
        }
    }
    .map_err(Into::into)
}

fn query_config(storage: &dyn Storage) -> StdResult<ConfigResponse> {
    Config::load(storage).map(|Config { cadence_hours, .. }| ConfigResponse { cadence_hours })
}

fn query_reward(
    storage: &dyn Storage,
    querier: QuerierWrapper<'_>,
    env: &Env,
) -> ContractResult<Percent> {
    let config: Config = Config::load(storage)?;

    let lpps = config
        .protocols
        .iter()
        .map(|protocol| lpp_platform::new_stub(&protocol.lpp, querier, env));

    RewardCalculator::new(lpps, &config.tvl_to_apr).map(|calc| calc.apr())
}

fn try_dispatch(deps: DepsMut<'_>, env: &Env, timealarm: Addr) -> ContractResult<MessageResponse> {
    let now = env.block.time;

    let config = Config::load(deps.storage)?;
    let setup_alarm = setup_alarm(
        timealarm,
        now,
        Duration::from_hours(config.cadence_hours),
        deps.querier,
    )?;

    let last_dispatch = DispatchLog::last_dispatch(deps.storage);
    DispatchLog::update(deps.storage, env.block.time)?;

    let lpps = config
        .protocols
        .iter()
        .map(|protocol| lpp_platform::new_stub(&protocol.lpp, deps.querier, env));
    let oracles = config.protocols.iter().map(|protocol| {
        oracle_platform::new_unchecked_base_currency_stub::<_, UsdGroup>(
            protocol.oracle.clone(),
            deps.querier,
        )
    });

    cmd::dispatch(
        Period::from_till(last_dispatch, now),
        &config.tvl_to_apr,
        lpps,
        oracles,
        &config.treasury,
    )
    .map(|dispatch_res| dispatch_res.merge_with(MessageResponse::messages_only(setup_alarm)))
}

fn setup_alarm(
    timealarm: Addr,
    now: Timestamp,
    alarm_in: Duration,
    querier: QuerierWrapper<'_>,
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
            coins, from_json,
            testing::{mock_dependencies_with_balance, mock_env, mock_info},
            Addr, DepsMut,
        },
        testing::customized_mock_deps_with_contracts,
    };

    use crate::{
        contract::sudo,
        msg::{ConfigResponse, InstantiateMsg, Protocol, QueryMsg, SudoMsg},
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
            protocol: Protocol {
                lpp: Addr::unchecked(LPP_ADDR),
                oracle: Addr::unchecked(ORACLE_ADDR),
            },
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
        let value: ConfigResponse = from_json(res).unwrap();
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
        let value: ConfigResponse = from_json(res).unwrap();
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
        let value: ConfigResponse = from_json(res).unwrap();
        assert_eq!(value.cadence_hours, 20);
    }
}
