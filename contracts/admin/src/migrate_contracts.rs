use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage},
};

use crate::{
    common::{maybe_migrate_contract, type_defs::Contracts},
    error::ContractError,
    msg::MigrateContracts,
    state::{contracts as state_contracts, migration_release},
};

pub(crate) fn migrate(
    storage: &mut dyn Storage,
    admin_contract_address: Addr,
    msg: MigrateContracts,
) -> Result<Response, ContractError> {
    migration_release::store(storage, msg.release)?;

    let contracts_addrs: Contracts = state_contracts::load(storage)?;

    let mut batch: Batch = Batch::default();

    maybe_migrate_contract(&mut batch, admin_contract_address, msg.admin_contract);

    Ok(batch
        .merge(contracts_addrs.migrate(msg.migration_spec))
        .into())
}
