use sdk::cosmwasm_std::Storage;

mod config;
mod dispatch_log;

pub(super) fn wipe_out(storage: &mut dyn Storage) {
    config::wipe_out(storage);
    dispatch_log::wipe_out(storage);
}
