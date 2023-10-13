use platform::{batch::Batch, message::Response as MessageResponse};
use sdk::cosmwasm_std::{Addr, Storage};

use crate::result::ContractResult;
use crate::{
    common::{maybe_migrate_contract, type_defs::Contracts},
    msg::MigrateContracts,
    state::{contracts as state_contracts, migration_release},
};

pub(super) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_address: Addr,
    msg: MigrateContracts,
) -> ContractResult<MessageResponse> {
    migration_release::store(storage, msg.release)?;

    let contracts_addrs: Contracts = state_contracts::load(storage)?;

    let mut batch: Batch = Batch::default();

    maybe_migrate_contract(&mut batch, admin_contract_address, msg.admin_contract);

    Ok(batch
        .merge(contracts_addrs.clone().migrate(msg.migration_spec))
        .merge(contracts_addrs.post_migration_execute(msg.post_migration_execute))
        .into())
}
