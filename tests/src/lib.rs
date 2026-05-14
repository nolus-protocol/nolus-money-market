#![cfg(all(test, not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used)]

use common::test_case::TestCase;
use cw_time::IntoInstant;
use finance::instant::Instant;

mod common;
mod lease;
mod leaser;
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
) -> Instant {
    test_case.app.block_info().time.into_instant()
}
