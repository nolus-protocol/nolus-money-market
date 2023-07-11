use error::{Error, Result};
use sdk::{
    cosmwasm_std::{Addr, Storage},
    cw_storage_plus::Item,
};
use std::ops::{Deref, DerefMut};

mod contract_owner;
pub use contract_owner::ContractOwnerAccess;
pub mod error;

pub struct SingleUserAccess<'storage, 'namespace, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    storage: S,
    storage_item: Item<'namespace, Addr>,
}

impl<'storage, 'namespace, S> SingleUserAccess<'storage, 'namespace, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub const fn new(storage: S, storage_namespace: &'namespace str) -> Self {
        Self {
            storage,
            storage_item: Item::new(storage_namespace),
        }
    }

    pub fn check(&self, user: &Addr) -> Result {
        self.storage_item
            .load(self.storage.deref())
            .map_err(Into::into)
            .and_then(|granted_to| UserPermission::new(granted_to).check(user))
    }
}

impl<'storage, 'namespace, S> SingleUserAccess<'storage, 'namespace, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn grant_to(&mut self, user: &Addr) -> Result {
        self.storage_item
            .save(self.storage.deref_mut(), user)
            .map_err(Into::into)
    }
}

#[derive(Debug, Eq, PartialEq)]
struct UserPermission {
    user: Addr,
}

impl UserPermission {
    pub fn check(&self, user: &Addr) -> Result {
        if self.user == user {
            Ok(())
        } else {
            Err(Error::Unauthorized {})
        }
    }

    const fn new(user: Addr) -> Self {
        Self { user }
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{testing::MockStorage, Addr, Storage};

    use crate::{
        error::{Error, Result},
        SingleUserAccess, UserPermission,
    };

    const NAMESPACE: &str = "my-nice-permission";

    #[test]
    fn grant_check() {
        let mut storage = MockStorage::new();
        let storage_ref: &mut dyn Storage = &mut storage;
        let mut access = SingleUserAccess::new(storage_ref, NAMESPACE);
        let user = Addr::unchecked("cosmic address");

        assert!(access.check(&user).is_err());
        access.grant_to(&user).unwrap();
        access.check(&user).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let mut storage = MockStorage::new();
        let storage_ref: &dyn Storage = &mut storage;
        let access = SingleUserAccess::new(storage_ref, NAMESPACE);
        let not_authorized = Addr::unchecked("hacker");

        assert!(matches!(
            access.check(&not_authorized).unwrap_err(),
            Error::Std(_)
        ));
    }

    #[test]
    fn check() {
        const ADDRESS: &str = "admin";

        check_permission(ADDRESS, ADDRESS).unwrap();
    }

    #[test]
    fn check_fail() {
        assert_eq!(
            Error::Unauthorized {},
            check_permission("user12", "user21").unwrap_err(),
        );
    }

    fn check_permission(granted_to: &str, asked_for: &str) -> Result {
        UserPermission::new(Addr::unchecked(granted_to)).check(&Addr::unchecked(asked_for))
    }
}
