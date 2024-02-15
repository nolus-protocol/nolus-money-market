use sdk::{cosmwasm_ext::as_dyn::storage, cosmwasm_std::Addr, cw_storage_plus::Item};

pub use self::contract_owner::ContractOwnerAccess;
use self::error::{Error, Result};

mod contract_owner;
pub mod error;

pub fn check(permitted_to: &Addr, accessed_by: &Addr) -> Result {
    if permitted_to == accessed_by {
        Ok(())
    } else {
        Err(Error::Unauthorized {})
    }
}

pub struct SingleUserAccess<'namespace, S>
where
    S: storage::Dyn,
{
    storage: S,
    storage_item: Item<'namespace, Addr>,
}

impl<'namespace, S> SingleUserAccess<'namespace, S>
where
    S: storage::Dyn,
{
    pub const fn new(storage: S, storage_namespace: &'namespace str) -> Self {
        Self {
            storage,
            storage_item: Item::new(storage_namespace),
        }
    }

    pub fn check(&self, user: &Addr) -> Result {
        self.storage_item
            .load(self.storage.as_dyn())
            .map_err(Into::into)
            .and_then(|granted_to| check(&granted_to, user))
    }
}

impl<'namespace, S> SingleUserAccess<'namespace, S>
where
    S: storage::DynMut,
{
    pub fn grant_to(&mut self, user: &Addr) -> Result {
        self.storage_item
            .save(self.storage.as_dyn_mut(), user)
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{testing::MockStorage, Addr};

    use crate::{
        error::{Error, Result},
        SingleUserAccess,
    };

    const NAMESPACE: &str = "my-nice-permission";

    #[test]
    fn grant_check() {
        let mut access = SingleUserAccess::new(MockStorage::new(), NAMESPACE);
        let user = Addr::unchecked("cosmic address");

        assert!(access.check(&user).is_err());
        access.grant_to(&user).unwrap();
        access.check(&user).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let access = SingleUserAccess::new(MockStorage::new(), NAMESPACE);
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
        super::check(&Addr::unchecked(granted_to), &Addr::unchecked(asked_for))
    }
}
