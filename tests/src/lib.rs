#![cfg(all(test, not(target_arch = "wasm32")))]

mod dispatcher_tests;

mod profit_tests;

mod lpp_tests;

mod common;

mod leaser_tests;

mod lease_tests;

mod oracle_tests;

mod timealarms_tests;

mod rust_runtime_tests;
