use currencies::Lpn;
use sdk::cosmwasm_std::Storage;

use crate::{
    contract::Result as ContractResult,
    state::{rewards, total},
};

pub fn migrate(store: &mut dyn Storage) -> ContractResult<()> {
    rewards::migrate_from_0_8_12::migrate(store)
        .and_then(|balance_nlpn| total::migrate_from_0_8_12::migrate::<Lpn>(store, balance_nlpn))
}
