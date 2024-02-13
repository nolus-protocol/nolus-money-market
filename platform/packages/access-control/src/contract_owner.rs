use sdk::{cosmwasm_ext::as_dyn::storage, cosmwasm_std::Addr};

use crate::{error::Result, SingleUserAccess};

const CONTRACT_OWNER_NAMESPACE: &str = "contract_owner";

pub struct ContractOwnerAccess<S>
where
    S: storage::Dyn,
{
    access: SingleUserAccess<'static, S>,
}

impl<S> ContractOwnerAccess<S>
where
    S: storage::Dyn,
{
    pub const fn new(storage: S) -> Self {
        Self {
            access: SingleUserAccess::new(storage, CONTRACT_OWNER_NAMESPACE),
        }
    }

    pub fn check(&self, user: &Addr) -> Result {
        self.access.check(user)
    }
}

impl<S> ContractOwnerAccess<S>
where
    S: storage::DynMut,
{
    pub fn grant_to(&mut self, user: &Addr) -> Result {
        self.access.grant_to(user)
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{testing::MockStorage, Addr};

    use crate::{error::Error, ContractOwnerAccess};

    #[test]
    fn grant_check() {
        let mut access = ContractOwnerAccess::new(MockStorage::new());
        let user = Addr::unchecked("happy user");

        assert!(access.check(&user).is_err());
        access.grant_to(&user).unwrap();
        access.check(&user).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let access = ContractOwnerAccess::new(MockStorage::new());
        let not_authorized = Addr::unchecked("hacker");

        assert!(matches!(
            access.check(&not_authorized).unwrap_err(),
            Error::Std(_)
        ));
    }
}
