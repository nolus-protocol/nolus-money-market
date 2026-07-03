use access_control::permissions::DexResponseSafeDeliveryPermission;
use cw_time::IntoInstant;
use finance::duration::Duration;
use platform::{
    contract::{self, Validator},
    error as platform_error,
    message::Response as MessageResponse,
    response,
};
use sdk::{
    api::SudoMsg,
    cosmwasm_ext::Response as CwResponse,
    cosmwasm_std::{
        Api, Binary, Deps, DepsMut, Env, MessageInfo, QuerierWrapper, Reply, Storage, entry_point,
    },
};
use versioning::{
    ProtocolMigrationMessage, ProtocolPackageRelease, UpdatablePackage as _, VersionSegment,
    package_name, package_version,
};

use crate::{
    api::{ExecuteMsg, MigrateMsg, open::NewLeaseContract, query::QueryMsg},
    contract::api::Contract,
    error::{ContractError, ContractResult},
};

use super::state::{self, Response, State};

const CONTRACT_STORAGE_VERSION: VersionSegment = 10;
const CURRENT_RELEASE: ProtocolPackageRelease = ProtocolPackageRelease::current(
    package_name!(),
    package_version!(),
    CONTRACT_STORAGE_VERSION,
);

#[entry_point]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    info: MessageInfo,
    new_lease: NewLeaseContract,
) -> ContractResult<CwResponse> {
    //TODO move the following validations into the deserialization
    deps.api.addr_validate(new_lease.finalizer.as_str())?;
    deps.api.addr_validate(new_lease.form.customer.as_str())?;

    let addr_validator = contract::validator(deps.querier);
    addr_validator.check_contract(&new_lease.form.time_alarms)?;
    addr_validator.check_contract(&new_lease.form.market_price_oracle)?;
    addr_validator.check_contract(&new_lease.form.loan.lpp)?;
    addr_validator.check_contract(&new_lease.form.loan.profit)?;

    state::new_lease(deps.querier, info, new_lease)
        .and_then(|(batch, next_state)| state::save(deps.storage, &next_state).map(|()| batch))
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn migrate(
    deps: DepsMut<'_>,
    _env: Env,
    ProtocolMigrationMessage {
        migrate_from,
        to_release,
        message: MigrateMsg {},
    }: ProtocolMigrationMessage<MigrateMsg>,
) -> ContractResult<CwResponse> {
    // A pre-v10 (v9) lease persisted a state shape the v10 layout cannot load:
    // the remote-lease reshape made `remote_lease_id` / `remote_lease_controller`
    // required, turned `Account.host` optional, and added the `OpeningUnwind`
    // variant and per-currency drain baselines. Such a record has no meaningful
    // `remote_lease_id` to synthesise, so it is refused outright. In-family
    // upgrades (v10 -> v10.x -> v11) keep the storage version and run the
    // standard software update the leaser's `migrate_leases` batch drives, as
    // every sibling contract does.
    //
    // The storage gate must precede `update_software`: that path checks code
    // monotonicity first, so a real older-semver v9 lease would surface as
    // `OlderPackageCode` instead of the deliberate `UnsupportedMigration`.
    if migrate_from.same_storage(&CURRENT_RELEASE) {
        migrate_from
            .update_software(&CURRENT_RELEASE, &to_release)
            .map_err(ContractError::UpdateSoftware)
            .map(|()| response::empty_response())
    } else {
        Err(ContractError::UnsupportedMigration)
    }
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn reply(deps: DepsMut<'_>, env: Env, msg: Reply) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| lease.reply(deps.querier, env, msg))
        .map(response::response_only_messages)
        .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| {
        process_execute(msg, lease, deps.querier, env, info)
    })
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<CwResponse> {
    process_lease(deps.storage, |lease| {
        process_sudo(msg, lease, deps.api, deps.querier, env)
    })
    .map(response::response_only_messages)
    .inspect_err(platform_error::log(deps.api))
}

#[entry_point]
pub fn query(deps: Deps<'_>, env: Env, msg: QueryMsg) -> ContractResult<Binary> {
    match msg {
        QueryMsg::State { due_projection } => state::load(deps.storage)
            .and_then(|state| {
                state.state(
                    env.block.time.into_instant(),
                    Duration::from_secs(due_projection),
                    deps.querier,
                )
            })
            .and_then(|resp| cosmwasm_std::to_json_binary(&resp).map_err(Into::into)),
        QueryMsg::ProtocolPackageRelease {} => {
            cosmwasm_std::to_json_binary(&CURRENT_RELEASE).map_err(Into::into)
        }
    }
    .inspect_err(platform_error::log(deps.api))
}

fn process_lease<ProcFn>(
    storage: &mut dyn Storage,
    process_fn: ProcFn,
) -> ContractResult<MessageResponse>
where
    ProcFn: FnOnce(State) -> ContractResult<Response>,
{
    state::load(storage).and_then(process_fn).and_then(
        |Response {
             response,
             next_state,
         }| state::save(storage, &next_state).map(|()| response),
    )
}

fn process_execute(
    msg: ExecuteMsg,
    state: State,
    querier: QuerierWrapper<'_>,
    env: Env,
    info: MessageInfo,
) -> ContractResult<Response> {
    match msg {
        ExecuteMsg::Repay() => state.repay(querier, env, info),
        ExecuteMsg::ChangeClosePolicy(change) => {
            state.change_close_policy(change, querier, env, info)
        }
        ExecuteMsg::ClosePosition(spec) => state.close_position(spec, querier, env, info),
        ExecuteMsg::TimeAlarm {} => state.on_time_alarm(querier, env, info),
        ExecuteMsg::PriceAlarm() => state.on_price_alarm(querier, env, info),
        ExecuteMsg::DexCallback() => {
            access_control::check(
                &DexResponseSafeDeliveryPermission::new(&env.contract),
                &info,
            )?;
            state.on_dex_inner(querier, env)
        }
        ExecuteMsg::RemoteLeaseCallback(callback) => {
            state.on_remote_lease_callback(callback, info, querier, env)
        }
        ExecuteMsg::Heal() => state.heal(querier, env, info),
    }
}

fn process_sudo(
    msg: SudoMsg,
    state: State,
    api: &dyn Api,
    querier: QuerierWrapper<'_>,
    env: Env,
) -> ContractResult<Response> {
    match msg {
        SudoMsg::Response { request: _, data } => state.on_dex_response(data, querier, env),
        SudoMsg::Error {
            request: _,
            details,
        } => {
            let resp = details.into();
            api.debug(&format!("SudoMsg::Error({resp})"));
            state.on_dex_error(resp, querier, env)
        }
        SudoMsg::Timeout { request: _ } => state.on_dex_timeout(querier, env),
        // The lease funds over the ICS-20 transfer channel and never registers
        // an ICA, so it can never receive an `OpenAck`.
        SudoMsg::OpenAck { .. } => Err(ContractError::unsupported_operation("open ica response")),
    }
}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use sdk::cosmwasm_std::testing::{mock_dependencies, mock_env};
    use versioning::{
        ProtocolMigrationMessage, ProtocolPackageRelease, ProtocolPackageReleaseId, ReleaseId,
        VersionSegment, package_name, package_version,
    };

    use crate::{api::MigrateMsg, error::ContractError};

    use super::{CONTRACT_STORAGE_VERSION, migrate};

    #[test]
    fn migrate_in_family_runs_update_software() {
        let mut deps = mock_dependencies();
        let res = migrate(
            deps.as_mut(),
            mock_env(),
            migrate_msg(CONTRACT_STORAGE_VERSION),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn migrate_pre_v10_storage_rejected() {
        const PRE_V10_STORAGE: VersionSegment = CONTRACT_STORAGE_VERSION - 1;

        let mut deps = mock_dependencies();
        let err = migrate(deps.as_mut(), mock_env(), migrate_msg(PRE_V10_STORAGE)).unwrap_err();
        assert!(
            matches!(err, ContractError::UnsupportedMigration),
            "got {err:?}",
        );
    }

    fn migrate_msg(from_storage: VersionSegment) -> ProtocolMigrationMessage<MigrateMsg> {
        const SOFTWARE_ID: &str = env!("SOFTWARE_RELEASE_ID");
        const PROTOCOL_ID: &str = env!("PROTOCOL_RELEASE_ID");

        ProtocolMigrationMessage {
            migrate_from: ProtocolPackageRelease::current(
                package_name!(),
                package_version!(),
                from_storage,
            ),
            to_release: ProtocolPackageReleaseId::new(
                ReleaseId::new_test(SOFTWARE_ID),
                ReleaseId::new_test(PROTOCOL_ID),
            ),
            message: MigrateMsg {},
        }
    }
}
