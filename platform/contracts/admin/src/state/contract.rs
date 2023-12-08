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

    pub(crate) fn migrate(storage: &mut dyn Storage) -> StdResult<()> {
        const OLD_STORE: Item<'_, String> = Item::new("migration_release");

        let release = OLD_STORE.load(storage)?;

        OLD_STORE.remove(storage);

        STORE.save(storage, &Self::Migration { release })
    }

    pub(crate) fn load(storage: &dyn Storage) -> StdResult<Self> {
        STORE.load(storage)
    }
}
