use std::ops::{Deref, DerefMut};

use sdk::{
    cosmwasm_std::{Addr, MessageInfo, Storage},
    cw_storage_plus::Item,
};

use self::error::{Error, Result};
use self::permissions::SingleUserPermission;

pub mod error;
pub mod permissions;

pub struct Sender<'a> {
    pub addr: &'a Addr,
}

impl<'a> Sender<'a> {
    pub fn new(info: &'a MessageInfo) -> Self {
        Self { addr: &info.sender }
    }

    pub fn from_addr(addr: &'a Addr) -> Self {
        Self { addr }
    }
}

pub trait AccessPermission {
    fn granted_to(&self, sender: &Sender<'_>) -> bool;
}

/// Checks if access is granted to the given caller.
pub fn check<P>(permission: &P, sender: &Sender<'_>) -> Result
where
    P: AccessPermission + ?Sized,
{
    if permission.granted_to(sender) {
        Ok(())
    } else {
        Err(Error::Unauthorized {})
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, ContractInfo, Storage, testing::MockStorage};

    use crate::{
        error::{Error, Result},
        permissions::{SameContractOnly, SingleUserPermission},
    };

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
        let address = Addr::unchecked("hacker");
        let sender = Sender::from_addr(&address);

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
        let address = Addr::unchecked(asked_for);
        let sender = Sender::from_addr(&address);

        super::check(
            &SingleUserPermission::new(&Addr::unchecked(granted_to)),
            &sender,
        )
    }
}
