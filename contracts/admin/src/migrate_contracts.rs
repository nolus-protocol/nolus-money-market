use std::iter;

use finance::currency::SymbolOwned;
use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Binary, StdResult, Storage, WasmMsg},
};

use crate::{
    common::{
        type_defs::{
            MaybeMigrateGeneralContract, MaybeMigrateLpnContract, MigrateContract,
            MigrateGeneralContract, MigrateLpnContract, MigrateLpnContracts,
        },
        Contracts as _, GeneralContracts, LpnContracts,
    },
    error::ContractError,
    msg::MigrateContracts,
    state::{
        contracts::{self as state_contracts, LpnContractsAddrsIter},
        migration_release,
    },
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_address: Addr,
    msg: Box<MigrateContracts>,
) -> Result<Response, ContractError> {
    migration_release::store(storage, msg.release)?;

    let general_contracts_addrs: GeneralContracts<Addr> = state_contracts::load_general(storage)?;
    let mut lpn_contracts_addrs_iter: LpnContractsAddrsIter<'_> =
        state_contracts::load_lpn_contracts(storage);

    let mut batch: Batch = Batch::default();

    iter::once((admin_contract_address, msg.admin_contract))
        .chain(general_contracts_addrs.zip_iter(msg.general_contracts))
        .filter_map(
            |(addr, code_id_with_msg): (Addr, MaybeMigrateGeneralContract)| {
                code_id_with_msg
                    .map(|code_id_with_msg: MigrateGeneralContract| (addr, code_id_with_msg))
            },
        )
        .for_each(add_to_batch(&mut batch));

    lpn_contracts_addrs_iter.try_for_each(migrate_lpn_contracts(msg.lpn_contracts, &mut batch))?;

    Ok(batch.into())
}

fn add_to_batch(batch: &mut Batch) -> impl FnMut((Addr, MigrateContract)) + '_ {
    |(addr, migrate): (Addr, MigrateContract)| {
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

fn migrate_lpn_contracts(
    mut contracts: MigrateLpnContracts,
    batch: &mut Batch,
) -> impl FnMut(StdResult<(SymbolOwned, LpnContracts<Addr>)>) -> Result<(), ContractError> + '_ {
    let mut add_to_batch = add_to_batch(batch);

    move |result: StdResult<(SymbolOwned, LpnContracts<Addr>)>| -> Result<(), ContractError> {
        let (symbol, group): (SymbolOwned, LpnContracts<Addr>) = result?;

        group
            .zip_iter(contracts.as_mut())
            .filter_map(
                |(addr, maybe_migrate): (Addr, &mut MaybeMigrateLpnContract)| {
                    maybe_migrate
                        .as_mut()
                        .map(|migrate: &mut MigrateLpnContract| (addr, migrate))
                },
            )
            .try_for_each(|(addr, migrate): (Addr, &mut MigrateLpnContract)| {
                migrate
                    .migrate_msg
                    .remove(&symbol)
                    .map(|migrate_msg: String| {
                        add_to_batch((
                            addr,
                            MigrateContract {
                                code_id: migrate.code_id,
                                migrate_msg,
                            },
                        ))
                    })
                    .ok_or(ContractError::MissingMigrationMessages {
                        symbol: symbol.clone(),
                    })
            })
    }
}
