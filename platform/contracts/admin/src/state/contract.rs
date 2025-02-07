use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct ExpectedInstantiation {
    code_id: CodeId,
    address: Addr,
}

impl ExpectedInstantiation {
    const STORE: Item<ExpectedInstantiation> = Item::new("expected_instantiation");

    pub(crate) const fn new(code_id: CodeId, address: Addr) -> Self {
        Self { code_id, address }
    }

    pub(crate) const fn code_id(&self) -> CodeId {
        self.code_id
    }

    pub(crate) const fn address(&self) -> &Addr {
        &self.address
    }

    pub(crate) fn into_address(self) -> Addr {
        self.address
    }

    pub(crate) fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        debug_assert!(!Self::STORE.exists(storage));

        Self::STORE.save(storage, self)
    }

    pub(crate) fn load(storage: &dyn Storage) -> StdResult<Self> {
        Self::STORE.load(storage)
    }

    pub(crate) fn clear(storage: &mut dyn Storage) {
        debug_assert!(Self::STORE.exists(storage));

        Self::STORE.remove(storage)
    }
}
