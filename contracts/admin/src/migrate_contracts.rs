use platform::batch::Batch;
use sdk::{
    cosmwasm_ext::Response,
    cosmwasm_std::{Addr, Storage},
};

use crate::{
    common::{maybe_migrate_contract, GeneralContracts, LpnContracts, MigrateContracts as _},
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

    let general_contracts_addrs: GeneralContracts<Addr> = state_contracts::load_general(storage)?;
    let lpn_contracts_addrs: LpnContracts<Addr> = state_contracts::load_lpn_contracts(storage)?;

    let mut batch: Batch = Batch::default();

    maybe_migrate_contract(&mut batch, admin_contract_address, msg.admin_contract);

    Ok(batch
        .merge(general_contracts_addrs.migrate(msg.general_contracts)?)
        .merge(lpn_contracts_addrs.migrate(msg.lpn_contracts)?)
        .into())
}
