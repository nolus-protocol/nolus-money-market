use std::ops::{Deref, DerefMut};

use sdk::cosmwasm_std::{Addr, MessageInfo, Storage};

use crate::{Sender, SingleUserAccess, error::Result};

const CONTRACT_OWNER_NAMESPACE: &str = "contract_owner";

pub struct ContractOwnerAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    access: SingleUserAccess<'storage, S>,
}

impl<'storage, S> ContractOwnerAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage>,
{
    pub const fn new(storage: S) -> Self {
        Self {
            access: SingleUserAccess::new(storage, CONTRACT_OWNER_NAMESPACE),
        }
    }

    pub fn check(&self, info: &MessageInfo) -> Result {
        self.access.check(&Sender::new(info))
    }
}

impl<'storage, S> ContractOwnerAccess<'storage, S>
where
    S: Deref<Target = dyn Storage + 'storage> + DerefMut,
{
    pub fn grant_to(&mut self, user: &Addr) -> Result {
        self.access.grant_to(user)
    }
}

#[cfg(test)]
mod tests {
    use sdk::cosmwasm_std::{Addr, MessageInfo, Storage, testing::MockStorage};

    use crate::{ContractOwnerAccess, error::Error};

    #[test]
    fn grant_check() {
        let mut storage = MockStorage::new();
        let storage_ref: &mut dyn Storage = &mut storage;
        let mut access = ContractOwnerAccess::new(storage_ref);
        let user = Addr::unchecked("happy user");
        let msg_info = MessageInfo {
            sender: user.clone(),
            funds: vec![],
        };

        assert!(access.check(&msg_info).is_err());
        access.grant_to(&user).unwrap();
        access.check(&msg_info).unwrap();
    }

    #[test]
    fn check_no_grant() {
        let mut storage = MockStorage::new();
        let storage_ref: &dyn Storage = &mut storage;
        let access = ContractOwnerAccess::new(storage_ref);
        let not_authorized = MessageInfo {
            sender: Addr::unchecked("hacker"),
            funds: vec![],
        };

        assert!(matches!(
            access.check(&not_authorized).unwrap_err(),
            Error::Std(_)
        ));
    }
}
