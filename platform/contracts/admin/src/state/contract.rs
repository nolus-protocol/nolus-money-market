use serde::{Deserialize, Serialize};

use platform::contract::CodeId;
use sdk::{
    cosmwasm_ext::as_dyn::storage,
    cosmwasm_std::{Addr, StdResult},
    cw_storage_plus::Item,
};
use versioning::ReleaseLabel;

const STORE: Item<'_, Contract> = Item::new("contract_state_machine");

#[derive(Serialize, Deserialize)]
pub(crate) enum Contract {
    AwaitContractsMigrationReply {
        release: ReleaseLabel,
    },
    Instantiate {
        expected_code_id: CodeId,
        expected_address: Addr,
    },
}

impl Contract {
    pub(crate) fn store<S>(&self, storage: &mut S) -> StdResult<()>
    where
        S: storage::DynMut + ?Sized,
    {
        STORE.save(storage.as_dyn_mut(), self)
    }

    pub(crate) fn load<S>(storage: &S) -> StdResult<Self>
    where
        S: storage::Dyn + ?Sized,
    {
        STORE.load(storage.as_dyn())
    }

    pub(crate) fn clear<S>(storage: &mut S)
    where
        S: storage::DynMut + ?Sized,
    {
        STORE.remove(storage.as_dyn_mut())
    }
}
