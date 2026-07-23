use std::slice;

use currencies::PaymentGroup;
use currency::CurrencyDef;
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    fraction::{Fraction, Unit},
    percent::{Percent, Percent100},
    price::{self, Price},
    ratio::Ratio,
    rational::Rational,
    zero::Zero,
};
use lease::api::{
    ExecuteMsg,
    query::{ClosePolicy, StateResponse, opened::Status, paid::ClosingTrx},
};
use platform::coin_legacy;
use sdk::{cosmwasm_std::Addr, cw_multi_test::AppResponse, testing};

use crate::{
    common::{
        self, CwCoin, USER, ibc, lease as common_lease,
        leaser::{self as leaser_mod, Instantiator as LeaserInstantiator},
        remote_lease_controller_stub::SwapFill,
        swap,
        test_case::{TestCase, response::ResponseWithInterChainMsgs},
    },
    lease::heal,
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, PaymentCoin, PaymentCurrency,
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

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ super::price_lpn_of(),
        )
        .unwrap(),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    )
    .unwrap();
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

    let lease_amount = (Percent::from_permille(1000) + max_ltd)
        .of(price::total(
            downpayment,
            Price::<PaymentCurrency, LeaseCurrency>::identity(),
        )
        .unwrap())
        .unwrap();
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

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ super::price_lpn_of(),
        )
        .unwrap(),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    )
    .unwrap();

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
    let lease_position = price::total(
        price::total(downpayment + borrowed, super::price_lpn_of()).unwrap(),
        super::price_lpn_of().inv(),
    )
    .unwrap();

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
    repay_swap(test_case, lease, payment).unwrap_response()
}

pub(crate) fn repay_full<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle>(
    test_case: &mut TestCase<ProtocolsRegistry, Treasury, Profit, Reserve, Addr, Lpp, Oracle, Addr>,
    lease: Addr,
    payment: PaymentCoin,
    expected_funds: LeaseCoin,
    excess_balance: LpnCoin,
) -> AppResponse {
    let repay_response = repay_swap(test_case, lease.clone(), payment).ignore_response();
    expect_started_closing(repay_response, expected_funds);
    expect_paid(test_case, lease.clone(), expected_funds);
    expect_lease_amounts(test_case, lease.clone(), excess_balance);
    finish_closing(test_case, lease, expected_funds)
}

/// Drive a repay through the remote-lease swap: the customer's `payment` is
/// transferred to the remote, swapped to LPN (identity fill), and the proceeds
/// transferred back in. Returns the transfer-in response — for a full repay it
/// already carries the position-close `submit_tx`, mirroring the legacy flow.
pub(crate) fn repay_swap<ProtocolsRegistry, Treasury, Profit, Reserve, Leaser, Lpp, Oracle>(
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
) -> ResponseWithInterChainMsgs<'_, AppResponse> {
    let controller = test_case.address_book.remote_lease_controller().clone();
    // Identity DEX fill: the swap yields the payment's LPN value.
    swap::set_fill(&mut test_case.app, &controller, SwapFill::InputAmount);

    let mut response = send_payment_and_transfer(test_case, lease.clone(), payment);

    let paid = price::total(payment, super::price_lpn_of()).unwrap();

    // The buy-LPN swap fired inline on the payment's transfer-in ack; the lease
    // is now in TransferInInit and has emitted the transfer-in of the proceeds.
    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );
    assert_eq!(transfer_amount, coin_legacy::to_cosmwasm_on_dex(paid));
    let _ = response.unwrap_response();

    // Fidelity: the emitted swap must carry the payment as its input coin.
    let captured = swap::captured(&test_case.app, &controller);
    assert_eq!(
        <Coin<PaymentCurrency> as Into<CoinDTO<PaymentGroup>>>::into(payment),
        swap::token_in(&captured),
    );

    let lease_ica = TestCase::stub_pda(1);
    swap::deliver_transfer_in(&mut test_case.app, lease_ica, lease, &transfer_amount)
}

pub(crate) fn send_payment_and_transfer<
    ProtocolsRegistry,
    Treasury,
    Profit,
    Reserve,
    Leaser,
    Lpp,
    Oracle,
    PaymentC,
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
    lease_addr: Addr,
    payment: Coin<PaymentC>,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    PaymentC: CurrencyDef,
{
    let payment_cw: CwCoin = common::cwcoin(payment);
    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .execute(
            testing::user(USER),
            lease_addr.clone(),
            &ExecuteMsg::Repay {},
            slice::from_ref(&payment_cw),
        )
        .unwrap()
        .ignore_response();

    let ica_addr: Addr = TestCase::stub_pda(1);

    let transfer_amount: CwCoin = ibc::expect_transfer(
        &mut response,
        TestCase::LEASER_IBC_CHANNEL,
        lease_addr.as_str(),
        ica_addr.as_str(),
    );

    assert_eq!(transfer_amount, payment_cw);

    () = response.unwrap_response();

    ibc::do_transfer(
        &mut test_case.app,
        lease_addr,
        ica_addr,
        false,
        &transfer_amount,
    )
}

fn expect_started_closing(
    mut repay_response: ResponseWithInterChainMsgs<'_, ()>,
    expected_funds: LeaseCoin,
) {
    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut repay_response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    assert_eq!(
        transfer_amount,
        coin_legacy::to_cosmwasm_on_dex(expected_funds)
    );

    () = repay_response.unwrap_response();
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
        in_progress: ClosingTrx::TransferInInit,
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
    excess_balance: LpnCoin,
) {
    // The remote (StubPda) collateral is `expected_funds`, pinned with exact
    // equality by `expect_started_closing` (the close `submit_tx` transfers
    // exactly `to_cosmwasm_on_dex(expected_funds)` out of the remote) and by
    // `finish_closing` (the customer's balance grows by exactly
    // `expected_funds`). Both are Cosmos-observable, so the non-bech32 remote
    // balance need not be read here.
    common_lease::assert_lease_balance_eq(&test_case.app, &lease, common::cwcoin(excess_balance));
}

fn finish_closing<ProtocolsRegistry, Treasury, Profit, Reserve, Lpp, Oracle, TimeAlarms>(
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
) -> AppResponse {
    let customer_addr: Addr = testing::user(USER);
    let ica_addr: Addr = TestCase::stub_pda(1);

    let user_balance: LeaseCoin =
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap();

    // The position's collateral is returned from the remote — credit the
    // stand-in with it (the remote holds the asset) and run the transfer-in.
    let app_resp = swap::deliver_transfer_in(
        &mut test_case.app,
        ica_addr,
        lease.clone(),
        &coin_legacy::to_cosmwasm_on_dex(expected_funds),
    )
    .unwrap_response();

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
