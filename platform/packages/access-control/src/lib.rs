use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Addr, MessageInfo, Storage},
    cw_storage_plus::Item,
};

pub use self::contract_owner::ContractOwnerAccess;
use self::error::{Error, Result};
use self::permissions::SingleUserPermission;

mod contract_owner;
pub mod error;
pub mod permissions;

pub struct Sender<'a> {
    addr: &'a Addr,
}

impl<'a> Sender<'a> {
    pub fn new(info: &'a MessageInfo) -> Self {
        Self { addr: &info.sender }
    }
    
    pub fn from_addr(addr: &'a Addr) -> Self {
        Self { addr }
    }
}

impl<'info> AsRef<Addr> for Sender<'info> {
    fn as_ref(&self) -> &Addr {
        self.addr
    }
}

pub trait AccessPermission {
    fn granted_to(&self, sender: &Sender) -> bool;
}

/// Checks if access is granted to the given caller.
pub fn check<P>(permission: &P, sender: &Sender) -> Result
where
    P: AccessPermission + ?Sized,
{
    if permission.granted_to(sender) {
        Ok(())
    } else {
        Err(Error::Unauthorized {})
    }
}

pub struct SingleUserAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    storage: S,
    storage_item: Item<Addr>,
}

impl<'storage, S> SingleUserAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub const fn new(storage: S, storage_namespace: &'static str) -> Self {
        Self {
            storage,
            storage_item: Item::new(storage_namespace),
        }
    }

    pub fn check(&self, sender: &Sender) -> Result {
        self.storage_item
            .load(self.storage.deref())
            .map_err(Into::into)
            .and_then(|ref granted_to| {
                check(&SingleUserPermission::new(&granted_to), sender)
            })
    }
}

impl<'storage, S> SingleUserAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn grant_to(&mut self, user: &Addr) -> Result {
        self.storage_item
            .save(self.storage.deref_mut(), user)
            .map_err(Into::into)
    }

    pub fn revoke(&mut self) {
        self.storage_item.remove(self.storage.deref_mut())
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, ContractInfo, MessageInfo, Storage, testing::MockStorage};

    use crate::{
        Sender,
        SingleUserAccess,
        error::{Error, Result},
        permissions::{SameContractOnly, SingleUserPermission},
    };

    const NAMESPACE: &str = "my-nice-permission";

    #[test]
    fn grant_check() {
        let mut storage = MockStorage::new();
        let storage_ref: &mut dyn Storage = &mut storage;
        let mut access = SingleUserAccess::new(storage_ref, NAMESPACE);
        let sender = Sender::from_addr(&Addr::unchecked("cosmic address"));

        access.check(&sender).unwrap_err();
        access.grant_to(sender.addr).unwrap();
        access.check(&sender).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let mut storage = MockStorage::new();
        let storage_ref: &dyn Storage = &mut storage;
        let access = SingleUserAccess::new(storage_ref, NAMESPACE);
        let sender = Sender::from_addr(&Addr::unchecked("hacker")); 

        assert!(matches!(
            access.check(&sender).unwrap_err(),
            Error::Std(_)
        ));
    }

    #[test]
    fn check_addr() {
        const ADDRESS: &str = "admin";

        check_addr_permission(ADDRESS, ADDRESS).unwrap();
    }

    #[test]
    fn check_same_contract_only() {
        let address = Addr::unchecked("contract admin");
        let contract_info = ContractInfo {
            address: address.clone(),
        };
        let sender = Sender::from_addr(&address);

        super::check(&SameContractOnly::new(&contract_info), &sender).unwrap();
    }

    #[test]
    fn check_same_contract_only_fail() {
        let contract_info = ContractInfo {
            address: Addr::unchecked("contract admin"),
        };
        let sender = Sender::from_addr(&Addr::unchecked("hacker")); 

        let check_result = super::check(&SameContractOnly::new(&contract_info), &sender);
        assert!(matches!(check_result.unwrap_err(), Error::Unauthorized {}));
    }

    #[test]
    fn check_fail() {
        assert_eq!(
            Error::Unauthorized {},
            check_addr_permission("user12", "user21").unwrap_err(),
        );
    }

    fn check_addr_permission(granted_to: &str, asked_for: &str) -> Result {
        let sender = Sender::from_addr(&Addr::unchecked(asked_for)); 

        super::check(
            &SingleUserPermission::new(&Addr::unchecked(granted_to)),
            &sender,
        )
    }
}
