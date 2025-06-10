use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Addr, MessageInfo, Storage},
    cw_storage_plus::Item,
};

pub use self::contract_owner::ContractOwnerAccess;
use self::error::{Error, Result};

mod contract_owner;
pub mod error;
pub mod permissions;

pub trait AccessPermission {
    fn is_granted_to(&self, info: &MessageInfo) -> bool;
}

/// Checks if access is granted to the given caller.
pub fn check<P>(permission: &P, info: &MessageInfo) -> Result
where
    P: AccessPermission,
{
    if permission.is_granted_to(info) {
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

    pub fn check(&self, info: &MessageInfo) -> Result {
        self.storage_item
            .load(self.storage.deref())
            .map_err(Into::into)
            .and_then(|granted_to| {
                check(&permissions::SingleUserPermission::new(&granted_to), info)
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
        let user_info = MessageInfo {
            sender: Addr::unchecked("cosmic address"),
            funds: vec![],
        };

        assert!(access.check(&user_info).is_err());
        access.grant_to(&user).unwrap();
        access.check(&user_info).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let mut storage = MockStorage::new();
        let storage_ref: &dyn Storage = &mut storage;
        let access = SingleUserAccess::new(storage_ref, NAMESPACE);
        let not_authorized = MessageInfo {
            sender: Addr::unchecked("hacker"),
            funds: vec![],
        };

        assert!(matches!(
            access.check(&not_authorized).unwrap_err(),
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
        let msg_info = MessageInfo {
            sender: address.clone(),
            funds: vec![],
        };

        let _ = super::check(&SameContractOnly::new(&contract_info), &msg_info);
    }

    #[test]
    fn check_same_contract_only_fail() {
        let address = Addr::unchecked("contract admin");
        let contract_info = ContractInfo {
            address: address.clone(),
        };
        let msg_info = MessageInfo {
            sender: Addr::unchecked("hacker"),
            funds: vec![],
        };

        let check_result = super::check(&SameContractOnly::new(&contract_info), &msg_info);

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
        let msg_info = MessageInfo {
            sender: Addr::unchecked(asked_for),
            funds: vec![],
        };

        super::check(
            &SingleUserPermission::new(&Addr::unchecked(granted_to)),
            &msg_info,
        )
    }
}
