use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::{ensure, Addr, Order, StdError, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::common::{GeneralContractsGroup, SpecializedContractsGroup};

pub(crate) struct ContractGroups;

impl ContractGroups {
    const GENERAL: Item<'_, GeneralContractsGroup<Addr>> = Item::new("general_contracts_group");

    const SPECIALIZED: Map<'_, SymbolOwned, SpecializedContractsGroup<Addr>> =
        Map::new("specialized_contract_groups");

    pub(crate) fn store_contract_addrs<I>(
        storage: &mut dyn Storage,
        general_group: GeneralContractsGroup<Addr>,
        specialized_groups_iter: I,
    ) -> StdResult<()>
    where
        I: IntoIterator<Item = (SymbolOwned, SpecializedContractsGroup<Addr>)>,
    {
        Self::GENERAL.save(storage, &general_group)?;

        specialized_groups_iter
            .into_iter()
            .try_for_each(|(symbol, group)| Self::SPECIALIZED.save(storage, symbol, &group))
    }

    pub(crate) fn add_specialized_group(
        storage: &mut dyn Storage,
        symbol: SymbolOwned,
        specialized_group: SpecializedContractsGroup<Addr>,
    ) -> StdResult<()> {
        Self::SPECIALIZED
            .update(
                storage,
                symbol,
                move |maybe_group: Option<SpecializedContractsGroup<Addr>>| {
                    ensure!(
                        maybe_group.is_none(),
                        StdError::generic_err("Group with this symbol already exists!")
                    );

                    Ok(specialized_group)
                },
            )
            .map(|_| ())
    }

    pub(crate) fn load_general(storage: &dyn Storage) -> StdResult<GeneralContractsGroup<Addr>> {
        Self::GENERAL.load(storage)
    }

    pub(crate) fn load_specialized(storage: &dyn Storage) -> SpecializedContractAddrsIter<'_> {
        Self::SPECIALIZED.range(storage, None, None, Order::Ascending)
    }
}

pub(crate) type SpecializedContractAddrsIter<'r> =
    Box<dyn Iterator<Item = StdResult<(SymbolOwned, SpecializedContractsGroup<Addr>)>> + 'r>;
