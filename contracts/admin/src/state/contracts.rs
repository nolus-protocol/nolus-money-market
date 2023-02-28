use sdk::{
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::common::type_defs::Contracts;

const CONTRACTS: Item<'_, Contracts> = Item::new("contracts");

pub(crate) fn store(storage: &mut dyn Storage, contracts: Contracts) -> StdResult<()> {
    CONTRACTS.save(storage, &contracts)
}

pub(crate) fn load(storage: &dyn Storage) -> StdResult<Contracts> {
    CONTRACTS.load(storage)
}
