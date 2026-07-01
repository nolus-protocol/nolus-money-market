//! Integration-harness address minting on the Nolus HRP.
//!
//! The shared `sdk::testing` helpers mint under the default `cosmwasm` prefix so
//! that crates whose unit tests pair a raw `mock_dependencies` with
//! `sdk::testing::user` stay bech32-consistent. The integration suites, however,
//! drive a single `cw-multi-test` app built on `sdk::testing::nolus_api` (see
//! [`crate::common::mock_app`]); every sender and every minted contract address it
//! canonicalizes must carry the Nolus HRP, and the profit drain vault so minted
//! must additionally pass the remote-profit `NolusReceiver` check. This module
//! re-exports the shared harness but overrides `user`/`contract` onto the Nolus
//! prefix, so the integration crate mints `nolus1…` addresses uniformly.

use sdk::cosmwasm_std::Addr;

pub use sdk::testing::{
    CwApp, CwContract, CwContractWrapper, InterChainMsgReceiver, InterChainMsgSender, new_app,
    new_inter_chain_msg_queue, nolus_api,
};

pub fn user(addr: &str) -> Addr {
    nolus_api().addr_make(addr)
}

pub fn contract(code_id: u64, instance_id: u64) -> Addr {
    user(&format!("contract_{code_id}_{instance_id}"))
}
