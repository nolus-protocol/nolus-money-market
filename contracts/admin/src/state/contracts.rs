use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

use crate::common::{GeneralContracts, LpnContracts};

const GENERAL_CONTRACTS: Item<'_, GeneralContracts<Addr>> = Item::new("general_contracts");

const LPN_CONTRACTS: Item<'_, LpnContracts<Addr>> = Item::new("lpn_contracts");

pub(crate) fn store(
    storage: &mut dyn Storage,
    general_contracts: GeneralContracts<Addr>,
    lpn_contracts: LpnContracts<Addr>,
) -> StdResult<()> {
    GENERAL_CONTRACTS.save(storage, &general_contracts)?;

    LPN_CONTRACTS.save(storage, &lpn_contracts)
}

pub(crate) fn load_general(storage: &dyn Storage) -> StdResult<GeneralContracts<Addr>> {
    GENERAL_CONTRACTS.load(storage)
}

pub(crate) fn load_lpn_contracts(storage: &dyn Storage) -> StdResult<LpnContracts<Addr>> {
    LPN_CONTRACTS.load(storage)
}
