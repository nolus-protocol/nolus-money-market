#![cfg(all(test, not(target_arch = "wasm32")))]
#![allow(clippy::unwrap_used)]
// The suite is feature-partitioned across the DEX axis (see the module gating
// below): each build compiles the whole shared `common` harness but runs only
// one half of the suites, so the other half's harness helpers are unused in any
// single build even though the union of both DEX configurations exercises them.
#![allow(dead_code)]

use common::test_case::TestCase;
use cw_time::IntoInstant;
use finance::instant::Instant;

mod common;

// Every suite swaps exclusively through a controller stand-in (the remote-lease
// controller for the lease suites, the remote-profit controller for
// `profit_tests` post-#652 PR4b), so the whole suite is DEX-agnostic and is
// compiled and run once under the placeholder DEX. `profit_tests` joined this
// axis when the profit contract retired its local-DEX ICA swap for the
// remote-swap transport — it no longer needs a real `dex-*` flag.
#[cfg(feature = "dex-test_impl")]
mod lease;
#[cfg(feature = "dex-test_impl")]
mod leaser;
#[cfg(feature = "dex-test_impl")]
mod lpp_tests;
#[cfg(feature = "dex-test_impl")]
mod oracle_tests;
#[cfg(feature = "dex-test_impl")]
mod profit_tests;
#[cfg(feature = "dex-test_impl")]
mod reserve;
#[cfg(feature = "dex-test_impl")]
mod timealarms_tests;
#[cfg(feature = "dex-test_impl")]
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
