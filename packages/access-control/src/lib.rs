use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

pub struct SingleUserAccess<'r> {
    storage_namespace: &'r str,
    address: Addr,
}

impl<'r> SingleUserAccess<'r> {
    pub const fn new(storage_namespace: &'r str, address: Addr) -> Self {
        Self {
            storage_namespace,
            address,
        }
    }

    pub fn load(storage: &dyn Storage, storage_namespace: &'r str) -> StdResult<Self> {
        Item::new(storage_namespace)
            .load(storage)
            .map(|address| Self {
                storage_namespace,
                address,
            })
    }

    pub fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        Item::new(self.storage_namespace).save(storage, &self.address)
    }

    pub const fn address(&self) -> &Addr {
        &self.address
    }

    pub fn check_access(&self, addr: &Addr) -> Result<(), Unauthorized> {
        if &self.address == addr {
            Ok(())
        } else {
            Err(Unauthorized)
        }
    }
}

impl<'r> From<SingleUserAccess<'r>> for Addr {
    fn from(value: SingleUserAccess<'r>) -> Self {
        value.address
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
#[error("[Access Control] Checked address doesn't match the one associated with access control variable!")]
pub struct Unauthorized;

#[cfg(test)]
mod tests {
    use super::*;
    use sdk::cosmwasm_std::testing::MockStorage;

    #[test]
    fn store_load() {
        const NAMESPACE: &str = "ownership";

        let mut storage = MockStorage::new();

        let original = SingleUserAccess::new(NAMESPACE, Addr::unchecked("cosmic address"));

        original.store(&mut storage).unwrap();

        let loaded = SingleUserAccess::load(&storage, NAMESPACE).unwrap();

        assert_eq!(loaded.storage_namespace, original.storage_namespace);
        assert_eq!(loaded.address, original.address);
    }

    #[test]
    fn load_fail() {
        const NAMESPACE: &str = "ownership";

        let storage = MockStorage::new();

        assert!(SingleUserAccess::load(&storage, NAMESPACE).is_err());
    }

    fn check_addr_template(store: &str, check: &str) -> Result<(), Unauthorized> {
        SingleUserAccess::new("ownership", Addr::unchecked(store))
            .check_access(&Addr::unchecked(check))
    }

    #[test]
    fn check() {
        const ADDRESS: &str = "cosmic address";

        check_addr_template(ADDRESS, ADDRESS).unwrap();
    }
    #[test]
    fn check_fail() {
        assert_eq!(
            check_addr_template("cosmic address", "osmotic address").unwrap_err(),
            Unauthorized
        );
    }
}
