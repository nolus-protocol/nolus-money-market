use serde::{Deserialize, Serialize};

use sdk::{cosmwasm_std::Storage, cw_storage_plus::Item};

const STORAGE: Item<'static, DispatchLog> = Item::new("dispatch_log");

#[derive(Serialize, Deserialize)]
struct DispatchLog {}

pub fn wipe_out(storage: &mut dyn Storage) {
    STORAGE.remove(storage)
}
