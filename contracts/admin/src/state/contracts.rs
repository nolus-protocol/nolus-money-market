use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

use crate::{common::type_defs::Contracts, result::ContractResult};

const CONTRACTS: Item<'_, Contracts> = Item::new("contracts");

pub(crate) fn store(storage: &mut dyn Storage, contracts: Contracts) -> ContractResult<()> {
    CONTRACTS.save(storage, &contracts).map_err(Into::into)
}

pub(crate) fn load(storage: &dyn Storage) -> ContractResult<Contracts> {
    CONTRACTS.load(storage).map_err(Into::into)
}
