use std::slice;

use crate::common::testing;
use currencies::PaymentGroup;
use finance::instant::Instant;
use finance::{
    coin::CoinDTO,
    duration::Duration,
    fraction::{Fraction, Unit},
    percent::{Percent, Percent100},
    price,
    ratio::Ratio,
    rational::Rational,
    zero::Zero,
};
use lease::api::{
    ExecuteMsg,
    query::{ClosePolicy, StateResponse, opened::Status, paid::ClosingTrx},
};
use platform::coin_legacy;
use sdk::{
    cosmwasm_std::{Addr, Event},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        self, CwCoin, USER, lease as common_lease,
        leaser::{self as leaser_mod, Instantiator as LeaserInstantiator},
        remote_lease_controller_stub as stub,
        test_case::{TestCase, app::App, response::ResponseWithInterChainMsgs},
    },
    lease::heal,
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseTestCase, LpnCoin, LpnCurrency, PaymentCoin, PaymentCurrency,
};

#[test]
fn partial_repay() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;

    let amount = super::quote_borrow(&test_case, downpayment).to_primitive();
    let partial_payment: PaymentCoin = Fraction::<PaymentCoin>::of(
        &Ratio::new(common::coin(1), common::coin(2)),
        super::create_payment_coin(amount),
    );

    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease = super::open_lease(&mut test_case, downpayment, None);

    repay_partial(&mut test_case, lease.clone(), partial_payment);

    let query_result = super::state_query(&test_case, lease);

    assert_eq!(query_result, expected_result);
}

#[test]
fn partial_repay_after_time() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;

    let lease = super::open_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Duration::from_nanos(
        LeaserInstantiator::REPAYMENT_PERIOD.nanos() >> 1,
    ));

    let query_result = super::state_query(&test_case, lease.clone());

    let StateResponse::Opened {
        overdue_margin,
        overdue_interest,
        due_margin,
        ..
    } = query_result
    else {
        unreachable!()
    };

    super::feed_price(&mut test_case);

    let due_margin_to_pay: LpnCoin = LpnCoin::try_from(due_margin)
        .unwrap()
        .checked_div(2)
        .unwrap();

    repay_partial(
        &mut test_case,
        lease.clone(),
        price::total(
            LpnCoin::try_from(overdue_margin).unwrap()
                + LpnCoin::try_from(overdue_interest).unwrap()
                + due_margin_to_pay,
            super::price_lpn_of::<PaymentCurrency>().inv(),
        )
        .unwrap(),
    );

    let query_result = super::state_query(&test_case, lease);

    if let StateResponse::Opened {
        overdue_margin,
        overdue_interest,
        ..
    } = query_result
    {
        assert!(
            overdue_margin.is_zero(),
            "Expected 0 for margin interest due, got {}",
            overdue_margin.amount()
        );

        assert!(
            overdue_interest.is_zero(),
            "Expected 0 for interest due, got {}",
            overdue_interest.amount()
        );
    } else {
        unreachable!()
    }
}

#[test]
#[should_panic = "InsufficientTransactionAmount"]
fn insufficient_payment() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;

    let lease = super::open_lease(&mut test_case, downpayment, None);

    let payment: PaymentCoin = super::create_payment_coin(49);
    repay_partial(&mut test_case, lease, payment);
}

#[test]
fn full_repay() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let borrowed_lpn = super::quote_borrow(&test_case, downpayment);
    let borrowed: PaymentCoin = price::total(borrowed_lpn, super::price_lpn_of().inv()).unwrap();

    let expected_amount: LeaseCoin = super::expected_opened_amount(downpayment, borrowed_lpn);
    repay_full(
        &mut test_case,
        lease.clone(),
        borrowed,
        expected_amount,
        LpnCoin::ZERO,
    );
}

#[test]
fn full_repay_with_max_ltd() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let max_ltd = Percent::from_percent(10);
    let borrowed = max_ltd.of(DOWNPAYMENT).unwrap();
    let lease = super::open_lease(&mut test_case, downpayment, Some(max_ltd));

    let lease_amount: LeaseCoin = super::expected_opened_amount(
        downpayment,
        price::total(borrowed, super::price_lpn_of()).unwrap(),
    );
    let expected_result = StateResponse::Opened {
        amount: lease_amount.into(),
        loan_interest_rate: Percent100::from_permille(70),
        margin_interest_rate: Percent100::from_permille(30),
        principal_due: price::total(max_ltd.of(downpayment).unwrap(), super::price_lpn_of())
            .unwrap()
            .into(),
        overdue_margin: LpnCoin::ZERO.into(),
        overdue_interest: LpnCoin::ZERO.into(),
        overdue_collect_in: LeaserInstantiator::REPAYMENT_PERIOD,
        due_margin: LpnCoin::ZERO.into(),
        due_interest: LpnCoin::ZERO.into(),
        due_projection: Duration::default(),
        close_policy: ClosePolicy::default(),
        validity: Instant::from_nanos(1537237459879305533),
        status: Status::Idle,
    };
    assert_eq!(
        expected_result,
        super::state_query(&test_case, lease.clone())
    );

    let expected_amount: LeaseCoin = lease_amount;

    repay_full(
        &mut test_case,
        lease.clone(),
        borrowed,
        expected_amount,
        LpnCoin::ZERO,
    );
}

#[test]
fn full_repay_with_excess() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let borrowed: PaymentCoin = price::total(
        super::quote_borrow(&test_case, downpayment),
        super::price_lpn_of().inv(),
    )
    .unwrap();
    let lease_position: LeaseCoin =
        super::expected_opened_amount(downpayment, super::quote_borrow(&test_case, downpayment));

    let overpayment = super::create_payment_coin(5);
    let overpayment_lpn = price::total(overpayment, super::price_lpn_of()).unwrap();
    let payment: PaymentCoin = borrowed + overpayment;

    repay_full(
        &mut test_case,
        lease.clone(),
        payment,
        lease_position,
        overpayment_lpn,
    );
}

/// Drive a partial repay over the controller transport with the swap and
/// the proceeds drain delayed, asserting the query inner tags at each stage
/// and that no ICA `SwapExactIn` is emitted.
#[test]
fn repay_swap_then_drain_inner_tags() {
    use lease::api::query::opened::{OngoingTrx, RepayTrx};

    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let amount = super::quote_borrow(&test_case, downpayment).to_primitive();
    let payment: PaymentCoin = Fraction::<PaymentCoin>::of(
        &Ratio::new(common::coin(1), common::coin(2)),
        super::create_payment_coin(amount),
    );

    let lease = super::open_lease(&mut test_case, downpayment, None);
    let controller = test_case.address_book.remote_lease_controller().clone();

    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        stub::op_tag::SWAP,
        stub::ResponseMode::Delayed,
    );
    stub::set_response_mode(
        &mut test_case.app,
        &controller,
        stub::op_tag::TRANSFER_OUT,
        stub::ResponseMode::Delayed,
    );

    // `send_repay` funds the swap via a transfer-out and acks it; the
    // controller swap is then held by the `Delayed` mode, so the lease sits
    // at `RepayTrx::Swap`.
    let _repay = send_repay(&mut test_case, lease.clone(), payment);
    assert_repay_in_progress(&test_case, &lease, RepayTrx::Swap);

    let _swap_ack =
        stub::deliver_pending_callback(&mut test_case.app, &controller, stub::op_tag::SWAP);
    assert_repay_in_progress(&test_case, &lease, RepayTrx::TransferOut);
    assert_eq!(
        1,
        stub::recorded_transfer_outs(&test_case.app, &controller, &lease).len()
    );

    let _drain_ack =
        stub::deliver_pending_callback(&mut test_case.app, &controller, stub::op_tag::TRANSFER_OUT);
    assert_repay_in_progress(&test_case, &lease, RepayTrx::TransferInFinish);

    // Land the swapped-out LPN and complete the proceeds drain.
    let transfers = stub::recorded_transfer_outs(&test_case.app, &controller, &lease);
    let proceeds: LpnCoin = LpnCoin::try_from(*transfers[0].amount()).expect("LPN proceeds");
    deposit_lpn_proceeds(&mut test_case, &lease, proceeds);
    let _arrival = deliver_funds_arrival_alarm(&mut test_case, lease.clone());

    match super::state_query(&test_case, lease) {
        StateResponse::Opened { .. } => (),
        other => panic!("expected the lease still opened after a partial repay, got {other:?}"),
    }

    fn assert_repay_in_progress(
        test_case: &LeaseTestCase,
        lease: &Addr,
        expected: lease::api::query::opened::RepayTrx,
    ) {
        match super::state_query(test_case, lease.clone()) {
            StateResponse::Opened {
                status: Status::InProgress(OngoingTrx::Repayment { in_progress, .. }),
                ..
            } => assert_eq!(expected, in_progress),
            other => panic!("expected a repayment in progress, got {other:?}"),
        }
    }
}

/// Repay funds the swap before it runs: the customer payment is transferred
/// to the lease's remote account first (mirroring the opening flow), and only
/// once that transfer acks is the swap scheduled. The swap-only repay this
/// replaced emitted no funding transfer and asked the remote account to swap
/// funds that never left the local contract.
#[test]
fn repay_funds_the_swap_before_swapping() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let amount = super::quote_borrow(&test_case, downpayment).to_primitive();
    let payment: PaymentCoin = Fraction::<PaymentCoin>::of(
        &Ratio::new(common::coin(1), common::coin(2)),
        super::create_payment_coin(amount),
    );

    let lease = super::open_lease(&mut test_case, downpayment, None);
    let ica_addr: Addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let payment_cw: CwCoin = common::cwcoin(payment);
    let mut response = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::Repay {},
            slice::from_ref(&payment_cw),
        )
        .unwrap();

    // The payment is pushed to the remote account before any swap is scheduled.
    let funded: CwCoin = common::ibc::take_transfer(&mut response, TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(payment_cw, funded);
    let _ = response.unwrap_response();

    // The swap leg appears only after the funding transfer is acknowledged.
    let _ = common::ibc::do_transfer(&mut test_case.app, lease.clone(), ica_addr, false, &funded)
        .unwrap_response();
    assert_swap_recorded(&test_case, &lease, payment);
}

pub(crate) fn repay_partial<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: Addr,
    payment: PaymentCoin,
) -> AppResponse {
    repay_with_hook_on_swap(test_case, lease, payment, no_op_hook).unwrap_response()
}

pub(crate) fn repay_full<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle>(
    test_case: &mut TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Addr, Lpp, Oracle, Addr>,
    lease: Addr,
    payment: PaymentCoin,
    expected_funds: LeaseCoin,
    excess_balance: LpnCoin,
) -> AppResponse {
    let repay_response =
        repay_with_hook_on_swap(test_case, lease.clone(), payment, no_op_hook).unwrap_response();
    expect_started_closing(test_case, &repay_response, &lease, expected_funds);
    expect_paid(test_case, lease.clone(), expected_funds);
    expect_lease_amounts(test_case, lease.clone(), expected_funds, excess_balance);
    finish_closing(test_case, lease, expected_funds)
}

fn no_op_hook(_app: &mut App) {}

pub(crate) fn repay_with_hook_on_swap<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    PreCloseHook,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: Addr,
    payment: PaymentCoin,
    pre_close_hook: PreCloseHook,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    PreCloseHook: FnOnce(&mut App),
{
    let remote_account: Addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let _swap_response = send_repay(test_case, lease.clone(), payment);
    assert_swap_recorded(test_case, &lease, payment);

    // `send_repay` funded the swap by transferring the payment to the remote
    // account; the swap spends it there, so drain it off (mirroring
    // `settle_remote_swaps_on_ica`'s `coin_in` move). The swapped-out LPN
    // returns below via the drain's `deposit_lpn_proceeds`.
    consume_repay_swap_input(test_case, &remote_account, payment);

    // The swap and the proceeds drain synthesise their controller
    // acknowledgments inline (the stand-in's default `Ok` mode), so by here
    // the repay drain's transfer-out has already acked and the lease awaits
    // the LPN proceeds' local arrival. No ICA leg, no DEX swap, no
    // remote-transfer back. The hook runs now - after the repay drain
    // settled, before the funds-arrival alarm pays the loan off and emits
    // the close-leg transfer-out - so a caller can target a `ResponseMode`
    // at the close transfer-out alone (it shares `op_tag::TRANSFER_OUT`
    // with the repay drain).
    pre_close_hook(&mut test_case.app);

    let proceeds = proceeds_recorded(test_case, &lease);
    deposit_lpn_proceeds(test_case, &lease, proceeds);

    deliver_funds_arrival_alarm_with_msgs(test_case, lease)
}

/// Send `ExecuteMsg::Repay` and acknowledge the funding transfer-out that
/// moves the payment to the lease's remote account before the swap, mirroring
/// the opening flow. Once the transfer acks, the swap leg is scheduled over
/// the remote-lease controller; in the stand-in's default `Ok` mode it (and
/// the proceeds drain) acknowledge inline.
pub(crate) fn send_repay<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: Addr,
    payment: PaymentCoin,
) -> AppResponse {
    let payment_cw: CwCoin = common::cwcoin(payment);
    let ica_addr: Addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let mut response = test_case
        .app
        .execute(
            testing::user(USER),
            lease.clone(),
            &ExecuteMsg::Repay {},
            slice::from_ref(&payment_cw),
        )
        .unwrap();

    let funded: CwCoin = common::ibc::take_transfer(&mut response, TestCase::LEASER_IBC_CHANNEL);
    assert_eq!(payment_cw, funded);
    let _ = response.unwrap_response();

    common::ibc::do_transfer(&mut test_case.app, lease, ica_addr, false, &funded).unwrap_response()
}

fn assert_swap_recorded<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>(
    test_case: &TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, Addr>,
    lease: &Addr,
    payment: PaymentCoin,
) {
    let controller = test_case.address_book.remote_lease_controller();
    let swaps = stub::recorded_swaps(&test_case.app, controller, lease);
    // The opening swaps are recorded against the same lease; the repay swap
    // is the latest single-coin leg carrying the payment into LPN.
    let repay_swap = swaps.last().expect("the repay swap to be recorded");
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(payment),
        repay_swap.coin_in()
    );
    assert_eq!(
        currency::dto::<LpnCurrency, PaymentGroup>(),
        repay_swap.min_out().currency()
    );
}

/// Model the remote swap consuming its funded input. The funding
/// transfer-out landed the payment on the lease's remote account; the swap
/// spends it there, so drain it off in the DEX representation (mirroring
/// `settle_remote_swaps_on_ica`'s `coin_in` move). The amount is the payment
/// the lease swapped, kept as the single source of truth via the assertion
/// in `assert_swap_recorded`.
pub(crate) fn consume_repay_swap_input<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    remote_account: &Addr,
    payment: PaymentCoin,
) {
    test_case
        .app
        .send_tokens(
            remote_account.clone(),
            testing::user(common::ADMIN),
            &[coin_legacy::to_cosmwasm_on_dex(payment)],
        )
        .unwrap();
}

/// The LPN proceeds the repay swap drains home, read off the latest
/// recorded controller transfer-out for the lease.
pub(crate) fn proceeds_recorded<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
>(
    test_case: &TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle, Addr>,
    lease: &Addr,
) -> LpnCoin {
    let controller = test_case.address_book.remote_lease_controller();
    let transfers = stub::recorded_transfer_outs(&test_case.app, controller, lease);
    let proceeds = transfers.last().expect("a recorded proceeds transfer-out");
    LpnCoin::try_from(*proceeds.amount()).expect("the proceeds in LPN")
}

/// Land the swapped-out LPN on the lease's local account, standing in for
/// the proceeds drain's ICS-20 arrival the controller acknowledged.
pub(crate) fn deposit_lpn_proceeds<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: &Addr,
    proceeds: LpnCoin,
) {
    test_case.send_funds_from_admin(lease.clone(), &[common::cwcoin(proceeds)]);
}

fn deliver_funds_arrival_alarm_with_msgs<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: Addr,
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    let time_alarms = test_case.address_book.time_alarms().clone();
    test_case
        .app
        .execute(time_alarms, lease, &ExecuteMsg::TimeAlarm {}, &[])
        .unwrap()
}

/// The drain rides the remote-lease controller: the stand-in acknowledges
/// the emitted `TransferOut` inline, so by the end of the repay
/// transaction the lease already awaits the funds' local arrival.
fn expect_started_closing<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>(
    test_case: &TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    repay_response: &AppResponse,
    lease: &Addr,
    expected_funds: LeaseCoin,
) {
    repay_response.assert_event(
        &Event::new("wasm-ls-close-transfer-out").add_attribute("stage", "funds-arrival"),
    );

    let transfer_outs = stub::recorded_transfer_outs(
        &test_case.app,
        test_case.address_book.remote_lease_controller(),
        lease,
    );
    // A full repay drains its swapped LPN proceeds home (the first
    // transfer-out) and, once the loan is paid, the freed lease asset (the
    // close transfer-out). The close leg is the last one and carries the
    // asset funds.
    assert_eq!(2, transfer_outs.len());
    assert_eq!(
        &CoinDTO::<PaymentGroup>::from(expected_funds),
        transfer_outs[1].amount()
    );
}

fn expect_paid<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
    expected_funds: LeaseCoin,
) {
    let expected_result = StateResponse::Closing {
        amount: LeaseCoin::into(expected_funds),
        in_progress: ClosingTrx::TransferInFinish,
    };
    assert_eq!(expected_result, super::state_query(test_case, lease));
}

fn expect_lease_amounts<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Addr,
        Lpp,
        Oracle,
        TimeAlarms,
    >,
    lease: Addr,
    expected_funds: LeaseCoin,
    excess_balance: LpnCoin,
) {
    common_lease::assert_lease_balance_eq(&test_case.app, &lease, common::cwcoin(excess_balance));

    common_lease::assert_lease_balance_eq(
        &test_case.app,
        &TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID),
        coin_legacy::to_cosmwasm_on_dex(expected_funds),
    );
}

fn finish_closing<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle>(
    test_case: &mut TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Addr, Lpp, Oracle, Addr>,
    lease: Addr,
    expected_funds: LeaseCoin,
) -> AppResponse {
    let customer_addr: Addr = testing::user(USER);
    let ica_addr: Addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let user_balance: LeaseCoin =
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap();

    // Mirror the acknowledged transfer onto the bank balances: the remote
    // account (stood in by the ICA address) escrows the asset and the
    // paired ICS-20 channel lands it on the lease's local account.
    test_case
        .app
        .send_tokens(
            ica_addr,
            testing::user(common::ADMIN),
            &[coin_legacy::to_cosmwasm_on_dex(expected_funds)],
        )
        .unwrap();
    test_case
        .app
        .send_tokens(
            testing::user(common::ADMIN),
            lease.clone(),
            &[common::cwcoin(expected_funds)],
        )
        .unwrap();

    let app_resp = deliver_funds_arrival_alarm(test_case, lease.clone());

    assert_eq!(
        StateResponse::Closed(),
        super::state_query(test_case, lease.clone()),
    );

    assert_eq!(
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap(),
        user_balance + expected_funds
    );

    leaser_mod::assert_no_leases(
        &test_case.app,
        test_case.address_book.leaser().clone(),
        customer_addr,
    );
    heal::heal_no_inconsistency(&mut test_case.app, lease, testing::user(USER));

    app_resp
}

/// Fire the funds-arrival poll the drain scheduled, delivered from the
/// time-alarms contract the way the production alarm dispatch would.
pub(crate) fn deliver_funds_arrival_alarm<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
>(
    test_case: &mut TestCase<
        ProtocolsRegistry,
        Treasury,
        Profit,
        Reserve,
        Leaser,
        Lpp,
        Oracle,
        Addr,
    >,
    lease: Addr,
) -> AppResponse {
    let time_alarms = test_case.address_book.time_alarms().clone();
    test_case
        .app
        .execute(time_alarms, lease, &ExecuteMsg::TimeAlarm {}, &[])
        .unwrap()
        .unwrap_response()
}
