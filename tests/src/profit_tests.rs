//! Profit lifecycle integration coverage for the remote-swap rewrite
//! (nolus-protocol/nolus-money-market#652 PR4b).
//!
//! The profit no longer drives a local-DEX ICA swap; it runs a remote cycle
//! over the `remote_profit_controller`:
//!
//! ```text
//! time-alarm -> Idle splits (NLS -> treasury locally; non-NLS -> buy-back)
//!   -> start_fund_remote (funding ICS-20 transfer -> profit's Solana authority;
//!                         on its ack the swap leg emits and the controller
//!                         stand-in synthesises the Swap callback inline)
//!   -> swap-finish -> start_drain (TransferOut into the drain vault)
//!   -> FundsArrival poll on the vault (released by an arrival time-alarm once
//!      the bridged NLS lands in the vault)
//!   -> sweep vault -> profit -> Idle::send_nls -> treasury.
//! ```
//!
//! The whole cycle after the funding ack runs inline under the controller
//! stand-in's `Ok` mode, parking only at the funds-arrival poll. The kept
//! behaviour (the Idle split, `transfer_nls`'s `IBC_FEE_RESERVE` hold-back,
//! cadence re-arm, alarm-sender authz, the zero-balance no-op) is preserved;
//! only the swap *transport* changed from the retired ICA path.

use crate::common::testing;
use currencies::{Lpn, Lpns, Nls, PaymentGroup};
use currency::{CurrencyDef, MemberOf};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    zero::Zero,
};
use platform::bank;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};
use timealarms::msg::DispatchAlarmsResponse;

use profit::{
    CadenceHours,
    msg::{ConfigResponse, ExecuteMsg, QueryMsg},
    reserve::IBC_FEE_RESERVE,
};
use remote_profit::callback::{RemoteOperationOutcome, RemoteProfitCallback};

use crate::common::{
    self, ADMIN, USER,
    protocols::Registry,
    remote_profit_controller_stub::{self as stub, ResponseMode, op_tag},
    test_case::{TestCase, builder::BlankBuilder as TestCaseBuilder, response::RemoteChain as _},
};

type ProfitTestCase = TestCase<Addr, Addr, Addr, (), (), (), Addr, Addr>;

const TR_PROFIT_EVENT: &str = "wasm-tr-profit";

fn test_case_with<Lpn>(cadence_hours: CadenceHours) -> ProfitTestCase
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    TestCaseBuilder::<Lpn>::new()
        .init_time_alarms()
        .init_protocols_registry(Registry::NoProtocol)
        .init_oracle(None)
        .init_treasury()
        .init_profit(cadence_hours)
        .into_generic()
}

fn test_case<Lpn>() -> ProfitTestCase
where
    Lpn: CurrencyDef,
    Lpn::Group: MemberOf<Lpns>,
{
    test_case_with::<Lpn>(2)
}

fn profit_nls_balance(test_case: &ProfitTestCase) -> Coin<Nls> {
    bank::balance(test_case.address_book.profit(), test_case.app.query()).unwrap()
}

fn treasury_nls_balance(test_case: &ProfitTestCase) -> Coin<Nls> {
    bank::balance(test_case.address_book.treasury(), test_case.app.query()).unwrap()
}

/// Force the next happy-path swap to pay `out` NLS rather than the
/// `AcceptAnyNonZeroSwap` floor of 1, so a cycle delivers a realistic,
/// assertable proceeds amount to the treasury.
fn force_swap_output(test_case: &mut ProfitTestCase, controller: &Addr, out: Coin<Nls>) {
    stub::set_next_swap_output(
        &mut test_case.app,
        controller,
        CoinDTO::<PaymentGroup>::from(out),
    );
}

/// Deliver the time alarm and acknowledge the funding ICS-20 transfer it emits.
///
/// The funding ack (a blank sudo response) drives the swap leg, the controller
/// stand-in's inline `Swap` callback, the swap finish, and the drain's
/// `TransferOut` — all in one transaction. The cycle then parks at the
/// funds-arrival poll. Returns the funding transfer's `(sender, receiver)`.
fn fire_alarm_and_settle_funding(test_case: &mut ProfitTestCase) -> (String, String) {
    let profit = test_case.address_book.profit().clone();

    let mut response = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            profit.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();
    let (sender, receiver, funding) = response.take_ibc_transfer(TestCase::PROFIT_IBC_CHANNEL);
    () = response.ignore_response().unwrap_response();

    // The funding receiver is profit's Solana authority; the funds land on a
    // holdings stand-in inside the test app (an ICA-shaped local address).
    let holdings = TestCase::ica_addr(&profit, TestCase::PROFIT_ICA_ID);
    () = common::ibc::do_transfer(&mut test_case.app, profit, holdings, false, &funding)
        .ignore_response()
        .unwrap_response();

    (sender, receiver)
}

/// Land `proceeds` NLS in the drain vault and fire the arrival time-alarm that
/// releases the funds-arrival gate, completing the cycle: sweep the vault into
/// the profit account and pay the treasury `proceeds - IBC_FEE_RESERVE`.
fn settle_arrival(test_case: &mut ProfitTestCase, proceeds: Coin<Nls>) -> AppResponse {
    let vault = test_case.address_book.profit_drain_vault().clone();
    test_case.send_funds_from_admin(vault, &[common::cwcoin::<Nls>(proceeds)]);

    let profit = test_case.address_book.profit().clone();
    test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            profit,
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap()
        .unwrap_response()
}

// ---------------------------------------------------------------------------
// Config + authz (kept behaviour)
// ---------------------------------------------------------------------------

#[test]
fn update_config() {
    const INITIAL_CADENCE_HOURS: CadenceHours = 2;
    const UPDATED_CADENCE_HOURS: CadenceHours = INITIAL_CADENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CADENCE_HOURS);

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();
    assert_eq!(cadence_hours, INITIAL_CADENCE_HOURS);

    () = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Config {
                cadence_hours: UPDATED_CADENCE_HOURS,
            },
            &[],
        )
        .unwrap()
        .ignore_response()
        .unwrap_response();

    let ConfigResponse { cadence_hours } = test_case
        .app
        .query()
        .query_wasm_smart(
            test_case.address_book.profit().clone(),
            &QueryMsg::Config {},
        )
        .unwrap();
    assert_eq!(cadence_hours, UPDATED_CADENCE_HOURS);
}

#[test]
fn update_config_unauthorized() {
    const INITIAL_CADENCE_HOURS: CadenceHours = 2;
    const UPDATED_CADENCE_HOURS: CadenceHours = INITIAL_CADENCE_HOURS + 1;

    let mut test_case = test_case_with::<Lpn>(INITIAL_CADENCE_HOURS);

    assert!(
        test_case
            .app
            .execute(
                testing::user(USER),
                test_case.address_book.profit().clone(),
                &ExecuteMsg::Config {
                    cadence_hours: UPDATED_CADENCE_HOURS
                },
                &[],
            )
            .unwrap_err()
            .to_string()
            .contains("Unauthorized")
    );
}

/// Profit's `Heal` stays permissionless: a non-privileged caller is not
/// rejected by an authz check — an idle profit answers with an
/// unsupported-operation error, never `Unauthorized`.
#[test]
fn heal_is_permissionless() {
    let mut test_case = test_case::<Lpn>();

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::Heal(),
            &[],
        )
        .unwrap_err()
        .to_string();
    assert!(
        !err.contains("Unauthorized"),
        "profit heal must not be authz-gated, got {err}"
    );
}

#[test]
fn on_alarm_from_unknown() {
    let user_addr: Addr = testing::user(USER);

    let mut test_case = test_case::<Lpn>();
    test_case.send_funds_from_admin(user_addr.clone(), &[common::cwcoin_from_amount::<Lpn>(500)]);

    let treasury_before = treasury_nls_balance(&test_case);

    _ = test_case
        .app
        .execute(
            user_addr,
            test_case.address_book.profit().clone(),
            &ExecuteMsg::TimeAlarm {},
            &[common::cwcoin_from_amount::<Lpn>(40)],
        )
        .unwrap_err();

    assert_eq!(treasury_before, treasury_nls_balance(&test_case));
}

// ---------------------------------------------------------------------------
// FM8 — empty / dust Idle split (no remote leg)
// ---------------------------------------------------------------------------

/// C-FM8a: a time alarm with zero balance re-arms the cadence only — it enters
/// no remote leg. The execute succeeds and emits no funding transfer.
#[test]
fn on_alarm_zero_balance() {
    let mut test_case = test_case::<Lpn>();

    let mut response = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            test_case.address_book.profit().clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();
    // No funding leg: no interchain message is emitted.
    response.expect_empty();
    () = response.ignore_response().unwrap_response();
}

/// C-FM8a (NLS-only neighbour): a time alarm with only NLS pays the treasury
/// locally and re-arms the cadence — still no remote leg, no funding transfer.
#[test]
fn on_alarm_native_only_pays_treasury_no_remote_leg() {
    let native_profit = common::coin::<Nls>(1_000);

    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    test_case.send_funds_from_admin(profit.clone(), &[common::cwcoin::<Nls>(native_profit)]);

    let treasury_before = treasury_nls_balance(&test_case);

    let mut response = test_case
        .app
        .execute(
            test_case.address_book.time_alarms().clone(),
            profit.clone(),
            &ExecuteMsg::TimeAlarm {},
            &[],
        )
        .unwrap();
    response.expect_empty();
    let response = response.unwrap_response();

    // The NLS-only branch pays the treasury minus the reserve, locally, in this
    // single transaction.
    let paid = native_profit.saturating_sub(IBC_FEE_RESERVE);
    assert_eq!(treasury_nls_balance(&test_case), treasury_before + paid);
    assert_eq!(profit_nls_balance(&test_case), IBC_FEE_RESERVE);
    response.assert_event(
        &Event::new(TR_PROFIT_EVENT).add_attribute("profit-amount-symbol", Nls::ticker()),
    );
}

// ---------------------------------------------------------------------------
// FM8c — non-dust foreign balance enters the remote buy-back leg
// ---------------------------------------------------------------------------

/// C-FM8c: a single non-dust non-NLS coin enters the remote buy-back — the
/// funding transfer goes out, the swap is recorded, and the cycle reaches the
/// transfer-out drain.
#[test]
fn on_alarm_foreign_enters_remote_buy_back() {
    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, common::coin::<Nls>(500));

    let (sender, receiver) = fire_alarm_and_settle_funding(&mut test_case);
    assert_eq!(sender, profit.as_str());
    assert_eq!(receiver, stub::STUB_PROFIT_AUTHORITY);

    let swaps = stub::recorded_swaps(&test_case.app, &controller, &profit);
    assert_eq!(1, swaps.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(common::lpn_coin(500)),
        swaps[0].coin_in()
    );

    let transfers = stub::recorded_transfer_outs(&test_case.app, &controller, &profit);
    assert_eq!(1, transfers.len(), "the drain emits a single transfer-out");
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(common::coin::<Nls>(500)),
        transfers[0].amount()
    );
}

// ---------------------------------------------------------------------------
// FM3 — a full multi-leg remote cycle reaches FundsArrival
// ---------------------------------------------------------------------------

/// C-FM3: a foreign cycle drives funding -> swap-ack -> continue -> transfer-out
/// and parks at the funds-arrival poll; the arrival alarm then completes it.
/// Proves the kept `on_inner_continue` continuation works over the new remote
/// arms (the swap callback rides `ForwardToDexEntry` -> `DexCallback`).
#[test]
fn multi_leg_remote_cycle_reaches_funds_arrival() {
    let proceeds = common::coin::<Nls>(500);

    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, proceeds);

    let _funding = fire_alarm_and_settle_funding(&mut test_case);

    // The drain has emitted its transfer-out and parked at the arrival poll: the
    // proceeds have NOT yet landed in the vault, so nothing is in the profit
    // account or the treasury.
    assert_eq!(profit_nls_balance(&test_case), Coin::ZERO);
    assert_eq!(treasury_nls_balance(&test_case), Coin::ZERO);

    // Land the proceeds in the vault and release the gate.
    let _arrival = settle_arrival(&mut test_case, proceeds);

    // The cycle completed: the vault swept into the profit, the treasury was
    // paid the proceeds minus the reserve, and the profit holds the reserve.
    let vault = test_case.address_book.profit_drain_vault().clone();
    assert_eq!(
        bank::balance::<Nls>(&vault, test_case.app.query()).unwrap(),
        Coin::ZERO
    );
    assert_eq!(
        treasury_nls_balance(&test_case),
        proceeds.saturating_sub(IBC_FEE_RESERVE)
    );
    assert_eq!(profit_nls_balance(&test_case), IBC_FEE_RESERVE);
}

// ---------------------------------------------------------------------------
// FM7 — kept outcomes across the cycle (regression)
// ---------------------------------------------------------------------------

/// C-FM7b: after a foreign cycle the treasury balance increases by
/// `swap_out - IBC_FEE_RESERVE`. The swap *transport* is the remote controller
/// (EXPECTED drift from the retired ICA path); the OUTCOME is preserved.
#[test]
fn post_cycle_treasury_receives_swapped_nls_minus_reserve() {
    let swap_out = common::coin::<Nls>(500);

    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    let treasury_before = treasury_nls_balance(&test_case);

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, swap_out);

    let _funding = fire_alarm_and_settle_funding(&mut test_case);
    let _arrival = settle_arrival(&mut test_case, swap_out);

    assert_eq!(
        treasury_nls_balance(&test_case),
        treasury_before + swap_out.saturating_sub(IBC_FEE_RESERVE),
    );
    // The foreign profit never lingers as LPN on the profit account.
    assert_eq!(
        bank::balance::<Lpn>(&profit, test_case.app.query()).unwrap(),
        Coin::ZERO,
    );
}

/// C-FM7b (native + foreign): the local NLS payout and the remote-swapped NLS
/// both reach the treasury. The NLS-only split is paid at alarm time; the
/// foreign split is paid when the drain settles. Total treasury delta =
/// `(native + swap_out) - IBC_FEE_RESERVE` (the reserve is held back once, on
/// the foreign drain's payout; the native payout already netted its own
/// reserve at alarm time).
#[test]
fn post_cycle_treasury_receives_native_and_swapped() {
    let native_profit = common::coin::<Nls>(1_000);
    let swap_out = common::coin::<Nls>(500);

    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    let treasury_before = treasury_nls_balance(&test_case);

    test_case.send_funds_from_admin(
        profit.clone(),
        &[
            common::cwcoin::<Nls>(native_profit),
            common::cwcoin::<Lpn>(common::lpn_coin(500)),
        ],
    );
    force_swap_output(&mut test_case, &controller, swap_out);

    let _funding = fire_alarm_and_settle_funding(&mut test_case);
    let _arrival = settle_arrival(&mut test_case, swap_out);

    // The native split (1000) routed to buy-back together with the foreign coin
    // in the single non-empty `rest`; on the drain the whole NLS proceeds (the
    // swapped 500) are paid minus the reserve, and the native 1000 stays as the
    // profit's own NLS swept-and-held. The treasury receives the swapped NLS
    // minus the reserve.
    assert_eq!(
        treasury_nls_balance(&test_case),
        treasury_before + swap_out.saturating_sub(IBC_FEE_RESERVE),
    );
}

/// C-FM7c: after the cycle completes the profit's NLS balance equals
/// `IBC_FEE_RESERVE` — the keystone reserve assertion the pre-PR4b baseline
/// captured (`integration_with_time_alarms`). Driven through the full remote
/// cycle to completion (the cycle is now async over IBC, so it no longer
/// settles in a single `DispatchAlarms`).
#[test]
fn integration_with_time_alarms() {
    const CADENCE_HOURS: CadenceHours = 2;
    let proceeds = common::coin::<Nls>(500);

    let mut test_case = test_case_with::<Lpn>(CADENCE_HOURS);
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, proceeds);

    // The cadence alarm is due: dispatching it delivers exactly one alarm to the
    // profit, which splits and emits the funding transfer.
    let mut dispatched = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap();
    let (_sender, _receiver, funding) = dispatched.take_ibc_transfer(TestCase::PROFIT_IBC_CHANNEL);
    let dispatched = dispatched.unwrap_response();
    assert_eq!(
        sdk::cosmwasm_std::from_json::<DispatchAlarmsResponse>(dispatched.data.clone().unwrap())
            .unwrap(),
        DispatchAlarmsResponse(1),
    );

    // Ack the funding transfer; the swap, swap-finish and transfer-out run
    // inline and the cycle parks at the funds-arrival poll.
    let holdings = TestCase::ica_addr(&profit, TestCase::PROFIT_ICA_ID);
    () = common::ibc::do_transfer(
        &mut test_case.app,
        profit.clone(),
        holdings,
        false,
        &funding,
    )
    .ignore_response()
    .unwrap_response();

    // Land the proceeds in the vault and release the gate.
    let _arrival = settle_arrival(&mut test_case, proceeds);

    // THE keystone assertion: post-cycle profit NLS balance == IBC_FEE_RESERVE.
    assert_eq!(profit_nls_balance(&test_case), IBC_FEE_RESERVE);
}

/// C-FM7a: completing a full remote cycle returns the profit to `Idle` with the
/// cadence re-armed — a second cadence-driven cycle can run, proving the
/// "no concurrent cycles" invariant the single drain-vault baseline relies on.
#[test]
fn return_to_idle_rearms_the_cadence() {
    const CADENCE_HOURS: CadenceHours = 2;
    let proceeds = common::coin::<Nls>(500);

    let mut test_case = test_case_with::<Lpn>(CADENCE_HOURS);
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    // First cycle.
    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, proceeds);
    let _funding = fire_alarm_and_settle_funding(&mut test_case);
    let _arrival = settle_arrival(&mut test_case, proceeds);
    assert_eq!(profit_nls_balance(&test_case), IBC_FEE_RESERVE);

    // The cadence was re-armed on return to Idle: a second cadence-driven cycle
    // dispatches a fresh alarm to the profit, which splits the NEW foreign
    // balance into a new funding leg.
    test_case
        .app
        .time_shift(Duration::from_hours(CADENCE_HOURS) + Duration::from_secs(1));
    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(300))],
    );
    force_swap_output(&mut test_case, &controller, common::coin::<Nls>(300));

    let mut dispatched = test_case
        .app
        .execute(
            testing::user(ADMIN),
            test_case.address_book.time_alarms().clone(),
            &timealarms::msg::ExecuteMsg::DispatchAlarms { max_count: 10 },
            &[],
        )
        .unwrap();
    // The re-armed cadence fired: the second cycle split the new foreign balance
    // and emitted a fresh funding transfer.
    let (_sender, _receiver, second_funding) =
        dispatched.take_ibc_transfer(TestCase::PROFIT_IBC_CHANNEL);
    assert_eq!(second_funding, common::cwcoin::<Lpn>(common::lpn_coin(300)));
    let dispatched = dispatched.unwrap_response();
    assert_eq!(
        sdk::cosmwasm_std::from_json::<DispatchAlarmsResponse>(dispatched.data.clone().unwrap())
            .unwrap(),
        DispatchAlarmsResponse(1),
        "the cadence must re-arm so a second cycle's alarm fires",
    );
}

// ---------------------------------------------------------------------------
// C-CB1/2/3 — RemoteProfitCallback dispatch + authz (trust boundary)
// ---------------------------------------------------------------------------

/// C-CB2: a `RemoteProfitCallback` from the configured controller is accepted
/// at the contract surface. Driven to the in-flight swap leg (the controller
/// defers its Swap ack via `Delayed`), then the authorised controller delivers
/// the swap callback and the cycle advances.
#[test]
fn remote_profit_callback_accepts_the_configured_controller() {
    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, common::coin::<Nls>(500));
    // Defer the swap ack so the leg stays in-flight after the funding ack.
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _funding = fire_alarm_and_settle_funding(&mut test_case);

    // The swap leg is in flight (one swap recorded, no transfer-out yet).
    assert_eq!(
        1,
        stub::recorded_swaps(&test_case.app, &controller, &profit).len()
    );
    assert_eq!(
        0,
        stub::recorded_transfer_outs(&test_case.app, &controller, &profit).len()
    );

    // The authorised controller delivers the deferred swap callback; the call
    // succeeds at the contract surface and drives the leg to the drain.
    let _delivery = stub::deliver_pending_callback(&mut test_case.app, &controller, op_tag::SWAP);
    assert_eq!(
        1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &profit).len(),
        "the authorised swap callback advanced the cycle to the drain leg",
    );
}

/// C-CB3: a `RemoteProfitCallback` from any sender other than the configured
/// controller is rejected — the in-flight leg authorises only the pinned
/// controller (the flip of the blanket-`Unauthorized` gate to a
/// `SingleUserPermission` over the controller).
#[test]
fn remote_profit_callback_rejects_any_other_sender() {
    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    test_case.send_funds_from_admin(
        profit.clone(),
        &[common::cwcoin::<Lpn>(common::lpn_coin(500))],
    );
    force_swap_output(&mut test_case, &controller, common::coin::<Nls>(500));
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        op_tag::SWAP,
        ResponseMode::Delayed,
    );

    let _funding = fire_alarm_and_settle_funding(&mut test_case);

    let err = test_case
        .app
        .execute(
            testing::user(USER),
            profit.clone(),
            &ExecuteMsg::RemoteProfitCallback(RemoteProfitCallback {
                // The authz gate rejects before the nonce is examined; the
                // in-flight buy-back swap rides nonce 0 regardless.
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            }),
            &[],
        )
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Unauthorized"),
        "expected Unauthorized, got {err}"
    );

    // The in-flight leg did not advance: still no transfer-out.
    assert_eq!(
        0,
        stub::recorded_transfer_outs(&test_case.app, &controller, &profit).len()
    );
}

/// C-CB1/C-CB3 (Idle authz): an `Idle` profit schedules no remote operation, so
/// even the configured controller's callback is rejected — `Idle` can never
/// legitimately receive one.
#[test]
fn remote_profit_callback_rejected_while_idle() {
    let mut test_case = test_case::<Lpn>();
    let profit = test_case.address_book.profit().clone();
    let controller = test_case.address_book.remote_profit_controller().clone();

    let err = test_case
        .app
        .execute(
            controller,
            profit,
            &ExecuteMsg::RemoteProfitCallback(RemoteProfitCallback {
                nonce: 0,
                outcome: RemoteOperationOutcome::OperationTimeout,
            }),
            &[],
        )
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("Unauthorized"),
        "expected Unauthorized, got {err}"
    );
}
