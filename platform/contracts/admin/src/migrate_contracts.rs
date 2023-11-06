use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::Storage;

use crate::{
    common::{maybe_migrate_contract, type_defs::ContractsGroupedByDex, CheckedAddr},
    msg::MigrateContracts,
    result::Result,
    state::{contracts as state_contracts, migration_release},
};

pub(super) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_addr: CheckedAddr,
    MigrateContracts::MigrateContracts {
        release,
        admin_contract,
        migration_spec,
        post_migration_execute,
    }: MigrateContracts,
) -> Result<MessageResponse> {
    migration_release::store(storage, release)?;

    let contracts_addrs: ContractsGroupedByDex = state_contracts::load(storage)?;

    let mut batch: Batch = Batch::default();

    maybe_migrate_contract(&mut batch, admin_contract_addr, admin_contract);

    contracts_addrs
        .clone()
        .migrate(migration_spec)
        .and_then(|migrate_batch: Batch| {
            contracts_addrs
                .post_migration_execute(post_migration_execute)
                .map(|post_migration_execute_batch: Batch| {
                    batch
                        .merge(migrate_batch)
                        .merge(post_migration_execute_batch)
                        .into()
                })
        })
}
