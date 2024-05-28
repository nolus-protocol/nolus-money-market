use serde::{Deserialize, Serialize};

use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

const STORAGE: Item<'static, Config> = Item::new("dispatcher_config");

#[derive(Serialize, Deserialize)]
struct Config {}

pub fn wipe_out(storage: &mut dyn Storage) {
    STORAGE.remove(storage)
}
