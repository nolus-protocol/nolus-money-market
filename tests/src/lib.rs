#![cfg(all(test, not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used)]

use common::test_case::TestCase;
use sdk::cosmwasm_std::Timestamp;

mod common;
mod lease;
mod leaser_tests;
mod lpp_tests;
mod oracle_tests;
mod profit_tests;
mod reserve;
mod timealarms_tests;
mod treasury_tests;

fn block_time<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
) -> Timestamp {
    test_case.app.block_info().time
}
