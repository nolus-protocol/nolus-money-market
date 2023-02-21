use finance::currency::SymbolOwned;
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Binary, StdResult, Storage, WasmMsg},
};

use crate::{
    common::{
        type_defs::{
            MaybeMigrateGeneral, MaybeMigrateSpecialized, MigrateGeneral, MigrateInfo,
            MigrateSpecialized, MigrateSpecializedContracts,
        },
        GeneralContractsGroup, SpecializedContractsGroup,
    },
    error::ContractError,
    msg::MigrateContracts,
    state::{ContractGroups, MigrationRelease, SpecializedContractAddrsIter},
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_address: Addr,
    msg: Box<MigrateContracts>,
) -> Result<Response, ContractError> {
    MigrationRelease::store(storage, msg.release)?;

    let general_group_addrs: GeneralContractsGroup<Addr> = ContractGroups::load_general(storage)?;
    let mut specialized_groups: SpecializedContractAddrsIter<'_> =
        ContractGroups::load_specialized(storage);

    let mut batch: Batch = Batch::default();

    [
        (admin_contract_address, msg.admin_contract),
        (general_group_addrs.profit, msg.general_contracts.profit),
        (
            general_group_addrs.timealarms,
            msg.general_contracts.timealarms,
        ),
        (general_group_addrs.treasury, msg.general_contracts.treasury),
    ]
    .into_iter()
    .filter_map(|(addr, code_id_with_msg): (Addr, MaybeMigrateGeneral)| {
        code_id_with_msg.map(|code_id_with_msg: MigrateGeneral| (addr, code_id_with_msg))
    })
    .for_each(add_to_batch(&mut batch));

    specialized_groups.try_for_each(migrate_specialized(msg.specialized_contracts, &mut batch))?;

    Ok(batch.into())
}

fn add_to_batch(batch: &mut Batch) -> impl FnMut((Addr, MigrateInfo)) + '_ {
    |(addr, migrate): (Addr, MigrateInfo)| {
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
    mut specialized_group: MigrateSpecializedContracts,
    batch: &mut Batch,
) -> impl FnMut(StdResult<(SymbolOwned, SpecializedContractsGroup<Addr>)>) -> Result<(), ContractError>
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
                |(addr, maybe_migrate): (Addr, &mut MaybeMigrateSpecialized)| {
                    maybe_migrate.as_mut().map(|migrate: &mut MigrateSpecialized| (addr, migrate))
                }
            )
            .try_for_each(
                |(addr, migrate): (Addr, &mut MigrateSpecialized)| {
                    migrate.migrate_msg
                        .remove(&symbol)
                        .map(
                            |migrate_msg: String| add_to_batch((
                                addr,
                                MigrateInfo { code_id: migrate.code_id, migrate_msg },
                            ))
                        )
                        .ok_or(ContractError::MissingMigrationMessages { symbol: symbol.clone() })
                }
            )
    }
}
