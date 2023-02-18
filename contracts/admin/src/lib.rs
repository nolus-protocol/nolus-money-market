use finance::currency::SymbolOwned;
use platform::batch::Batch;
#[cfg(feature = "contract-with-bindings")]
use sdk::cosmwasm_std::entry_point;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{
        ensure_eq, to_binary, Addr, Binary, DepsMut, Env, MessageInfo, Reply, StdResult, Storage,
        WasmMsg,
    },
};
use versioning::{version, VersionSegment};

use self::{
    common::{GeneralContractsGroup, SpecializedContractsGroup},
    error::ContractError,
    msg::{
        ContractsMigrateIndividual, ExecuteMsg, GeneralContractsMaybeMigrateIndividual,
        GeneralContractsMigrateIndividual, InstantiateMsg, MigrateContractsData, MigrateMsg,
        SpecializedContractsMaybeMigrate, SpecializedContractsMaybeMigrateIndividual,
        SpecializedContractsMigrateIndividual, SudoMsg,
    },
    state::{
        add_specialized_contracts_group, load_and_remove_migration_release,
        load_general_contract_addrs, load_specialized_contract_addrs, store_contract_addrs,
        store_migration_release, SpecializedContractAddrsIter,
    },
};

pub mod common;
pub mod error;
pub mod msg;
pub mod state;

// version info for migration info
// const CONTRACT_STORAGE_VERSION_FROM: VersionSegment = 0;
const CONTRACT_STORAGE_VERSION: VersionSegment = 0;

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn instantiate(
    deps: DepsMut<'_>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    versioning::initialize(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    platform::contract::validate_addr(&deps.querier, &msg.general_contracts.profit)?;
    platform::contract::validate_addr(&deps.querier, &msg.general_contracts.timealarms)?;
    platform::contract::validate_addr(&deps.querier, &msg.general_contracts.treasury)?;

    for group in msg.specialized_contracts.values() {
        platform::contract::validate_addr(&deps.querier, &group.dispatcher)?;
        platform::contract::validate_addr(&deps.querier, &group.leaser)?;
        platform::contract::validate_addr(&deps.querier, &group.lpp)?;
        platform::contract::validate_addr(&deps.querier, &group.oracle)?;
    }

    store_contract_addrs(
        deps.storage,
        msg.general_contracts,
        msg.specialized_contracts,
    )?;

    Ok(Response::default())
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn migrate(deps: DepsMut<'_>, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    versioning::update_software(deps.storage, version!(CONTRACT_STORAGE_VERSION))?;

    Ok(Response::default().set_data(to_binary(sdk::RELEASE_VERSION)?))
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn execute(
    deps: DepsMut<'_>,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        #[cfg(any(debug_assertions, test, feature = "admin_contract_exec"))]
        ExecuteMsg::LocalNetSudo { sudo: msg } => sudo(deps, env, msg),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn sudo(deps: DepsMut<'_>, env: Env, msg: SudoMsg) -> Result<Response, ContractError> {
    match msg {
        SudoMsg::RegisterSymbolGroup {
            symbol,
            specialized_contracts,
        } => {
            add_specialized_contracts_group(deps.storage, symbol, specialized_contracts)?;

            Ok(Response::default())
        }
        SudoMsg::Migrate(migrate_contracts_variant) => migrate_managed_contracts(
            deps.storage,
            env.contract.address,
            migrate_contracts_variant,
        ),
    }
}

#[cfg_attr(feature = "contract-with-bindings", entry_point)]
pub fn reply(deps: DepsMut<'_>, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    let expected_release: String = load_and_remove_migration_release(deps.storage)?;

    let reported_release: String =
        platform::reply::from_execute(msg)?.ok_or(ContractError::NoMigrationResponseData {})?;

    ensure_eq!(
        reported_release,
        expected_release,
        ContractError::WrongRelease {
            reported: reported_release,
            expected: expected_release
        }
    );

    Ok(Response::default())
}

fn migrate_managed_contracts(
    storage: &mut dyn Storage,
    admin_contract_address: Addr,
    migrate_contracts_variant: Box<MigrateContractsData>,
) -> Result<Response, ContractError> {
    fn add_to_batch(batch: &mut Batch) -> impl FnMut((Addr, ContractsMigrateIndividual)) + '_ {
        |(addr, migrate): (Addr, ContractsMigrateIndividual)| {
            batch.schedule_execute_on_success_reply(
                WasmMsg::Migrate {
                    contract_addr: addr.into_string(),
                    new_code_id: migrate.code_id,
                    msg: Binary(migrate.migrate_msg.into()),
                },
                0,
            )
        }
    }

    fn migrate_specialized(
        mut specialized_group: SpecializedContractsMaybeMigrate,
        batch: &mut Batch,
    ) -> impl FnMut(
        StdResult<(SymbolOwned, SpecializedContractsGroup<Addr>)>,
    ) -> Result<(), ContractError>
           + '_ {
        let mut add_to_batch = add_to_batch(batch);

        move |result: StdResult<(SymbolOwned, SpecializedContractsGroup<Addr>)>| -> Result<(), ContractError> {
            let (symbol, group): (SymbolOwned, SpecializedContractsGroup<Addr>) = result?;

            [
                (group.dispatcher, &mut specialized_group.dispatcher),
                (group.leaser, &mut specialized_group.leaser),
                (group.lpp, &mut specialized_group.lpp),
                (group.oracle, &mut specialized_group.oracle),
            ]
                .into_iter()
                .filter_map(
                    |(addr, maybe_migrate): (Addr, &mut SpecializedContractsMaybeMigrateIndividual)| {
                        maybe_migrate.as_mut().map(|migrate: &mut SpecializedContractsMigrateIndividual| (addr, migrate))
                    }
                )
                .try_for_each(
                    |(addr, migrate): (Addr, &mut SpecializedContractsMigrateIndividual)| {
                        migrate.migrate_msg
                            .remove(&symbol)
                            .map(
                                |migrate_msg: String| add_to_batch((
                                    addr,
                                    ContractsMigrateIndividual { code_id: migrate.code_id, migrate_msg },
                                ))
                            )
                            .ok_or(ContractError::MissingMigrationMessages { symbol: symbol.clone() })
                    }
                )
        }
    }

    store_migration_release(storage, migrate_contracts_variant.release)?;

    let general_group_addrs: GeneralContractsGroup<Addr> = load_general_contract_addrs(storage)?;
    let mut specialized_groups: SpecializedContractAddrsIter =
        load_specialized_contract_addrs(storage);

    let mut batch: Batch = Batch::default();

    [
        (
            admin_contract_address,
            migrate_contracts_variant.admin_contract,
        ),
        (
            general_group_addrs.profit,
            migrate_contracts_variant.general_contracts.profit,
        ),
        (
            general_group_addrs.timealarms,
            migrate_contracts_variant.general_contracts.timealarms,
        ),
        (
            general_group_addrs.treasury,
            migrate_contracts_variant.general_contracts.treasury,
        ),
    ]
    .into_iter()
    .filter_map(
        |(addr, code_id_with_msg): (Addr, GeneralContractsMaybeMigrateIndividual)| {
            code_id_with_msg
                .map(|code_id_with_msg: GeneralContractsMigrateIndividual| (addr, code_id_with_msg))
        },
    )
    .for_each(add_to_batch(&mut batch));

    specialized_groups.try_for_each(migrate_specialized(
        migrate_contracts_variant.specialized_contracts,
        &mut batch,
    ))?;

    Ok(batch.into())
}
