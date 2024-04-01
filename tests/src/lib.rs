#![cfg(all(test, not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used)]

mod common;
mod lease;
mod leaser_tests;
mod lpp_tests;
mod oracle_tests;
mod profit_tests;
mod reserve;
mod timealarms_tests;
mod treasury_tests;
