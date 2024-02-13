use sdk::{
    cosmwasm_ext::as_dyn::{storage, AsDyn, AsDynMut},
    cosmwasm_std::{StdResult, Storage},
    cw_storage_plus::Item,
};

const STORAGE_ITEM: Item<'_, String> = Item::new("migration_release");

pub(crate) fn store<S>(storage: &mut S, migration_release: String) -> StdResult<()>
where
    S: storage::DynMut + ?Sized,
{
    STORAGE_ITEM.save(storage.as_dyn_mut(), &migration_release)
}

pub(crate) fn load<S>(storage: &S) -> StdResult<String>
where
    S: storage::Dyn + ?Sized,
{
    STORAGE_ITEM.load(storage.as_dyn())
}
