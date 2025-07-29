use sdk::{
    cosmwasm_std::{Addr, MessageInfo},
};

use self::error::{Error, Result};
use self::user::User;

pub mod error;
pub mod permissions;
pub mod user;

pub trait AccessPermission {
    fn granted_to<U>(&self, user: &U) -> bool
    where
        U: User;
}

/// Checks if access is granted to the given user.
pub fn check<P, U>(permission: &P, user: &U) -> Result
where
    P: AccessPermission + ?Sized,
    U: User,
{
    if permission.granted_to(user) {
        Ok(())
    } else {
        Err(Error::Unauthorized {})
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, ContractInfo};

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

        super::check(&SameContractOnly::new(&contract_info), &address).unwrap();
    }

    #[test]
    fn check_same_contract_only_fail() {
        let contract_info = ContractInfo {
            address: Addr::unchecked("contract admin"),
        };
        let address = Addr::unchecked("hacker");

        let check_result = super::check(&SameContractOnly::new(&contract_info), &address);
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

        super::check(
            &SingleUserPermission::new(&Addr::unchecked(granted_to)),
            &address,
        )
    }
}
