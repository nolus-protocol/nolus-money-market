#![cfg(all(test, not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used)]
// Every suite compiles into a single DEX-agnostic build, but each suite exercises
// only the subset of the shared `common` harness it needs, so some helpers are
// unused from any one suite's perspective.
#![allow(dead_code)]

use common::test_case::TestCase;
use cw_time::IntoInstant;
use finance::instant::Instant;

mod common;

// Every suite swaps exclusively through a controller stand-in (the remote-lease
// controller for the lease suites, the remote-profit controller for
// `profit_tests`), so the whole suite is DEX-agnostic and compiles and runs once.
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
