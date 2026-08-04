//! Swap harness for the remote-lease transport.
//!
//! Post-refactor a lease swap is a plain `WasmMsg::Execute` to the remote-lease
//! controller (see `lease::contract::transport::remote_lease`), not an ICA
//! `submit_tx`. The controller stand-in
//! ([`super::remote_lease_controller_stub`]) synthesises the `OperationResponse`
//! ack in-process, so the whole swap round-trip resolves inline within the
//! transaction that drives the lease into `SwapExactIn` — there is no separate
//! "observe the swap packet, then answer it" step any more.
//!
//! This module is the ergonomic surface the lifecycle tests use to:
//! - pre-configure the ack the counterparty will pay ([`set_fill`]),
//! - read back the swap request the lease actually emitted ([`captured`],
//!   [`token_in`], [`min_out`]) for `token_in` / `min_out` assertions,
//! - and, for the local-output swaps (repay / close / liquidation), credit the
//!   lease with the returning proceeds and fire the time alarm that finishes
//!   the transfer-in ([`deliver_transfer_in`]).
//!
//! Error and delayed acks are driven through the controller stand-in directly
//! ([`super::remote_lease_controller_stub::set_response_mode`]).

use std::slice;

use currencies::PaymentGroup;
use finance::coin::{Amount, CoinDTO};
use lease::api::ExecuteMsg;
use remote_lease::swap::SwapParams;
use sdk::{
    cosmwasm_std::{Addr, Coin as CwCoin},
    cw_multi_test::AppResponse,
    testing,
};

use super::{
    ADMIN,
    remote_lease_controller_stub::{self as stub, SwapFill},
    test_case::{app::App, response::ResponseWithInterChainMsgs},
};

/// Configure the `amount_out` a happy-path swap ack pays back.
pub(crate) fn set_fill(app: &mut App, controller: &Addr, fill: SwapFill) {
    stub::set_swap_fill(app, controller, fill);
}

/// The `SwapParams` of the most recent swap request the lease emitted, as
/// captured by the stand-in.
#[track_caller]
pub(crate) fn captured(app: &App, controller: &Addr) -> SwapParams<PaymentGroup, PaymentGroup> {
    stub::captured_swap(app, controller)
}

/// The number of swap requests the lease has emitted, as counted by the
/// stand-in — lets a test pin the exact swap-message cardinality.
#[track_caller]
pub(crate) fn count(app: &App, controller: &Addr) -> u64 {
    stub::swap_count(app, controller)
}

/// The number of transfer-out (transfer-back) requests the lease has emitted,
/// as counted by the stand-in — the transfer-back mirror of [`count`].
#[track_caller]
pub(crate) fn transfer_out_count(app: &App, controller: &Addr) -> u64 {
    stub::transfer_out_count(app, controller)
}

/// The first input coin of a captured swap — the successor of the legacy
/// `SwapRequest::token_in`.
pub(crate) fn token_in(params: &SwapParams<PaymentGroup, PaymentGroup>) -> CoinDTO<PaymentGroup> {
    match params {
        SwapParams::One { coin_in, .. } => *coin_in,
        SwapParams::Two { coin_in_1, .. } => *coin_in_1,
    }
}

/// The `min_out` amount of a captured swap — the successor of the legacy
/// `SwapRequest::min_token_out`.
pub(crate) fn min_out(params: &SwapParams<PaymentGroup, PaymentGroup>) -> Amount {
    params.min_out().amount()
}

/// The `amount` (with currency) of the most recent transfer-out request the
/// lease emitted, as captured by the stand-in — the successor of the amount the
/// legacy transfer-in `submit_tx` carried.
pub(crate) fn captured_transfer_out(app: &App, controller: &Addr) -> CoinDTO<PaymentGroup> {
    *stub::captured_transfer_out(app, controller).amount()
}

/// Complete a lease transfer-in that is parked in `TransferInFinish`.
///
/// Post-refactor the lease requests the returning funds through the
/// controller's `TransferOut` (answered inline by the stand-in) and then polls
/// its own bank balance for their arrival, retrying on a 5s time alarm. This
/// mirrors the Solana vault's IBC transfer landing on Nolus by crediting the
/// lease directly with `proceeds` — the bank-denom coin the poll expects
/// (`common::cwcoin(amount_out)`) — then firing the lease's time alarm from
/// `time_alarms` so `TransferInFinish` re-runs its poll and finishes. Returns
/// the alarm response so the caller can assert the finish events and any
/// follow-up (e.g. a full repay's position close).
pub(crate) fn deliver_transfer_in<'r>(
    app: &'r mut App,
    time_alarms: Addr,
    lease: Addr,
    proceeds: &CwCoin,
) -> ResponseWithInterChainMsgs<'r, AppResponse> {
    app.send_tokens(
        testing::user(ADMIN),
        lease.clone(),
        slice::from_ref(proceeds),
    )
    .unwrap();

    app.execute(time_alarms, lease, &ExecuteMsg::TimeAlarm {}, &[])
        .unwrap()
}
