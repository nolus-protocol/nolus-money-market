use finance::currency::SymbolOwned;
use sdk::{
    cosmwasm_std::{ensure, StdError},
    cosmwasm_std::{Addr, Order, StdResult, Storage},
    cw_storage_plus::{Item, Map},
};

use crate::common::{GeneralContractsGroup, SpecializedContractsGroup};

const MIGRATION_RELEASE: Item<'_, String> = Item::new("migration_release");

const GENERAL_CONTRACTS_GROUP: Item<'_, GeneralContractsGroup<Addr>> =
    Item::new("general_contracts_group");

const SPECIALIZED_CONTRACT_GROUPS: Map<'_, SymbolOwned, SpecializedContractsGroup<Addr>> =
    Map::new("specialized_contract_groups");

pub(crate) fn store_migration_release(
    storage: &mut dyn Storage,
    migration_release: String,
) -> StdResult<()> {
    MIGRATION_RELEASE.save(storage, &migration_release)
}

pub(crate) fn load_and_remove_migration_release(storage: &mut dyn Storage) -> StdResult<String> {
    let release: String = MIGRATION_RELEASE.load(storage)?;

    MIGRATION_RELEASE.remove(storage);

    Ok(release)
}

pub(crate) fn store_contract_addrs<I>(
    storage: &mut dyn Storage,
    general_group: GeneralContractsGroup<Addr>,
    specialized_groups: I,
) -> StdResult<()>
where
    I: IntoIterator<Item = (SymbolOwned, SpecializedContractsGroup<Addr>)>,
{
    GENERAL_CONTRACTS_GROUP.save(storage, &general_group)?;

    specialized_groups
        .into_iter()
        .try_for_each(|(symbol, group)| SPECIALIZED_CONTRACT_GROUPS.save(storage, symbol, &group))
}

pub(crate) fn add_specialized_contracts_group(
    storage: &mut dyn Storage,
    symbol: SymbolOwned,
    specialized_group: SpecializedContractsGroup<Addr>,
) -> StdResult<()> {
    SPECIALIZED_CONTRACT_GROUPS
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

pub(crate) fn load_general_contract_addrs(
    storage: &dyn Storage,
) -> StdResult<GeneralContractsGroup<Addr>> {
    GENERAL_CONTRACTS_GROUP.load(storage)
}

pub(crate) type SpecializedContractAddrsIter<'r> =
    Box<dyn Iterator<Item = StdResult<(SymbolOwned, SpecializedContractsGroup<Addr>)>> + 'r>;

pub(crate) fn load_specialized_contract_addrs(
    storage: &dyn Storage,
) -> SpecializedContractAddrsIter<'_> {
    SPECIALIZED_CONTRACT_GROUPS.range(storage, None, None, Order::Ascending)
}
