use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::{ensure, Addr, Order, StdError, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::common::{GeneralContracts, LpnContracts};

const GENERAL_CONTRACTS: Item<'_, GeneralContracts<Addr>> = Item::new("general_contracts");

const LPN_CONTRACTS: Map<'_, SymbolOwned, LpnContracts<Addr>> = Map::new("lpn_contracts");

pub(crate) fn store<I>(
    storage: &mut dyn Storage,
    general_contracts: GeneralContracts<Addr>,
    lpn_contracts: I,
) -> StdResult<()>
where
    I: IntoIterator<Item = (SymbolOwned, LpnContracts<Addr>)>,
{
    GENERAL_CONTRACTS.save(storage, &general_contracts)?;

    lpn_contracts
        .into_iter()
        .try_for_each(|(symbol, group)| LPN_CONTRACTS.save(storage, symbol, &group))
}

pub(crate) fn register_lpn_contracts(
    storage: &mut dyn Storage,
    symbol: SymbolOwned,
    contracts: LpnContracts<Addr>,
) -> StdResult<()> {
    LPN_CONTRACTS
        .update(
            storage,
            symbol,
            move |maybe_contracts: Option<LpnContracts<Addr>>| {
                ensure!(
                    maybe_contracts.is_none(),
                    StdError::generic_err("Contracts with this LPN already exists!")
                );

                Ok(contracts)
            },
        )
        .map(|_| ())
}

pub(crate) fn load_general(storage: &dyn Storage) -> StdResult<GeneralContracts<Addr>> {
    GENERAL_CONTRACTS.load(storage)
}

pub(crate) fn load_lpn_contracts(
    storage: &dyn Storage,
) -> impl Iterator<Item = LpnContractsSymbolAddrsResult> + '_ {
    LPN_CONTRACTS.range(storage, None, None, Order::Ascending)
}

pub(crate) type LpnContractsSymbolAddrs = (SymbolOwned, LpnContracts<Addr>);
pub(crate) type LpnContractsSymbolAddrsResult = StdResult<LpnContractsSymbolAddrs>;
