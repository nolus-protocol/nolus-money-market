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

    pub fn into_address(self) -> Addr {
        self.address
    }

    pub fn address_mut(&mut self) -> &mut Addr {
        &mut self.address
    }

    pub fn check_access(&self, addr: &Addr) -> Result<(), Unauthorized> {
        if &self.address == addr {
            Ok(())
        } else {
            Err(Unauthorized)
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, thiserror::Error)]
#[error("[Platform~Access Control] Checked address doesn't match the one associated with access control variable!")]
pub struct Unauthorized;
