use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_std::{Addr, StdResult, Storage},
    cw_storage_plus::Item,
};

const STORE: Item<'_, Contract> = Item::new("contract_state_machine");

#[derive(Serialize, Deserialize)]
pub(crate) enum Contract {
    Migration {
        release: String,
    },
    Instantiate {
        expected_code_id: CodeId,
        expected_address: Addr,
    },
}

impl Contract {
    pub(crate) fn store(&self, storage: &mut dyn Storage) -> StdResult<()> {
        STORE.save(storage, self)
    }

    pub(crate) fn load(storage: &mut dyn Storage) -> StdResult<Self> {
        STORE.load(storage)
    }
}
