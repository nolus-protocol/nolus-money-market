use std::slice;

use ::swap::testing::SwapRequest;
use currencies::PaymentGroup;
use currency::CurrencyDef;
use finance::{
    coin::{Amount, Coin, CoinDTO},
    duration::Duration,
    percent::{Percent, Percent100},
    price::{self, Price},
    ratio::SimpleFraction,
    rational::Rational,
    zero::Zero,
};
use lease::api::{
    ExecuteMsg,
    query::{ClosePolicy, StateResponse, opened::Status, paid::ClosingTrx},
};
use platform::coin_legacy::to_cosmwasm_on_dex;
use sdk::{
    cosmwasm_std::{Addr, Timestamp},
    cw_multi_test::AppResponse,
    testing,
};

use crate::{
    common::{
        self, CwCoin, USER, cwcoin, ibc,
        leaser::{self as leaser_mod, Instantiator as LeaserInstantiator},
        swap::{self},
        test_case::{TestCase, app::App, response::ResponseWithInterChainMsgs},
    },
    lease::heal,
};

use super::{
    DOWNPAYMENT, LeaseCoin, LeaseCurrency, LeaseTestCase, LpnCoin, LpnCurrency, PaymentCoin,
    PaymentCurrency, price_lpn_of,
};

#[test]
fn partial_repay() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;

    let amount = super::quote_borrow(&test_case, downpayment);
    let partial_payment = SimpleFraction::new(PaymentCoin::new(1), PaymentCoin::new(2))
        .of(super::create_payment_coin(amount.into()))
        .unwrap();

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
        ),
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
#[should_panic = "[Lease] [Position] The transaction amount should worth at least"]
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
    let borrowed: PaymentCoin = price::total(borrowed_lpn, super::price_lpn_of().inv());

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ super::price_lpn_of(),
        ),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    );
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

    let lease_amount = (Percent::HUNDRED + max_ltd)
        .of(price::total(
            downpayment,
            Price::<PaymentCurrency, LeaseCurrency>::identity(),
        ))
        .unwrap();
    let expected_result = StateResponse::Opened {
        amount: lease_amount.into(),
        loan_interest_rate: Percent100::from_permille(70),
        margin_interest_rate: Percent100::from_permille(30),
        principal_due: price::total(max_ltd.of(downpayment).unwrap(), super::price_lpn_of()).into(),
        overdue_margin: LpnCoin::ZERO.into(),
        overdue_interest: LpnCoin::ZERO.into(),
        overdue_collect_in: LeaserInstantiator::REPAYMENT_PERIOD,
        due_margin: LpnCoin::ZERO.into(),
        due_interest: LpnCoin::ZERO.into(),
        due_projection: Duration::default(),
        close_policy: ClosePolicy::default(),
        validity: Timestamp::from_nanos(1537237459879305533),
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
        ),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    );

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
    );
    let lease_position = price::total(
        price::total(downpayment + borrowed, price_lpn_of()),
        price_lpn_of().inv(),
    );

    let overpayment = super::create_payment_coin(5);
    let overpayment_lpn = price::total(overpayment, super::price_lpn_of());
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
        repay_with_hook_on_swap(test_case, lease.clone(), payment, no_op_hook).ignore_response();
    expect_started_closing(repay_response, expected_funds);
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
    SwapHook,
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
    swap_pre_hook: SwapHook,
) -> ResponseWithInterChainMsgs<'_, AppResponse>
where
    SwapHook: FnOnce(&mut App),
{
    let response = send_payment_and_transfer(test_case, lease.clone(), payment);

    let paid = price::total(payment, super::price_lpn_of());

    let requests: Vec<SwapRequest<PaymentGroup, PaymentGroup>> = swap::expect_swap(
        response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
        |_| {},
    );

    assert!(!requests.is_empty());
    assert_eq!(
        <Coin<PaymentCurrency> as Into<CoinDTO::<PaymentGroup>>>::into(payment),
        requests[0].token_in
    );

    swap_pre_hook(&mut test_case.app);

    let lease_ica = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let mut response = swap::do_swap(
        &mut test_case.app,
        lease.clone(),
        lease_ica.clone(),
        requests.into_iter(),
        |amount_in, in_denom, out_denom| {
            assert_eq!(amount_in, payment.into());
            assert_eq!(in_denom, PaymentCurrency::dex());
            assert_eq!(out_denom, LpnCurrency::dex());

            paid.into()
        },
    );

    let transfer_amount: CwCoin = ibc::expect_remote_transfer(
        &mut response,
        TestCase::DEX_CONNECTION_ID,
        TestCase::LEASE_ICA_ID,
    );

    assert_eq!(transfer_amount, to_cosmwasm_on_dex(paid));

    _ = response.unwrap_response();

    ibc::do_transfer(&mut test_case.app, lease_ica, lease, true, &transfer_amount)
}

fn send_payment_and_transfer<
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

    let ica_addr: Addr = TestCase::ica_addr(&lease_addr, TestCase::LEASE_ICA_ID);

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

    assert_eq!(transfer_amount, to_cosmwasm_on_dex(expected_funds));

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
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_funds),
        in_progress: Some(ClosingTrx::TransferInInit),
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
    if !excess_balance.is_zero() {
        assert_eq!(
            test_case
                .app
                .query()
                .query_all_balances(lease.clone())
                .unwrap(),
            &[cwcoin::<LpnCurrency, Amount>(excess_balance.into())],
        )
    }

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID))
            .unwrap(),
        &[to_cosmwasm_on_dex(expected_funds)],
    );
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
    let ica_addr: Addr = TestCase::ica_addr(&lease, TestCase::LEASE_ICA_ID);

    let user_balance: LeaseCoin =
        platform::bank::balance(&customer_addr, test_case.app.query()).unwrap();

    let app_resp = ibc::do_transfer(
        &mut test_case.app,
        ica_addr,
        lease.clone(),
        true,
        &to_cosmwasm_on_dex(expected_funds),
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
