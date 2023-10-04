use finance::{
    coin::Amount,
    duration::Duration,
    fraction::Fraction,
    percent::Percent,
    price::{self, Price},
    ratio::Rational,
    zero::Zero,
};

use ::lease::api::{ExecuteMsg, StateResponse};

use sdk::{
    cosmwasm_std::{Addr, Binary, Coin as CwCoin, Timestamp},
    cw_multi_test::AppResponse,
};

use crate::{
    common::{
        cwcoin,
        leaser::Instantiator as LeaserInstantiator,
        test_case::{
            response::{RemoteChain as _, ResponseWithInterChainMsgs},
            TestCase,
        },
        ADMIN, USER,
    },
    lease,
};

use super::{
    dex, LeaseCoin, LeaseCurrency, Lpn, LpnCoin, PaymentCoin, PaymentCurrency, DOWNPAYMENT,
};

#[test]
fn partial_repay() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;

    let quote_result = super::quote_query(&test_case, downpayment);
    let amount: LpnCoin = quote_result.borrow.try_into().unwrap();
    let partial_payment = Fraction::<PaymentCoin>::of(
        &Rational::new(1, 2),
        super::create_payment_coin(amount.into()),
    );
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = super::open_lease(&mut test_case, downpayment, None);
    repay(
        &mut test_case,
        lease_address.clone(),
        partial_payment,
        quote_result.total.try_into().unwrap(),
    );

    let query_result = super::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn partial_repay_after_time() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;

    let lease_address = super::open_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Duration::from_nanos(
        LeaserInstantiator::REPAYMENT_PERIOD.nanos() >> 1,
    ));

    let query_result = super::state_query(&test_case, lease_address.as_ref());

    let StateResponse::Opened {
        amount: lease_amount,
        previous_margin_due,
        previous_interest_due,
        current_margin_due,
        ..
    } = query_result else {
        unreachable!()
    };

    super::feed_price(&mut test_case);

    let current_margin_to_pay: LpnCoin = LpnCoin::try_from(current_margin_due)
        .unwrap()
        .checked_div(2)
        .unwrap();

    repay(
        &mut test_case,
        lease_address.clone(),
        price::total(
            LpnCoin::try_from(previous_margin_due).unwrap()
                + LpnCoin::try_from(previous_interest_due).unwrap()
                + current_margin_to_pay,
            super::price_lpn_of::<PaymentCurrency>().inv(),
        ),
        lease_amount.try_into().unwrap(),
    );

    let query_result = super::state_query(&test_case, lease_address.as_str());

    if let StateResponse::Opened {
        previous_margin_due,
        previous_interest_due,
        ..
    } = query_result
    {
        assert!(
            previous_margin_due.is_zero(),
            "Expected 0 for margin interest due, got {}",
            previous_margin_due.amount()
        );

        assert!(
            previous_interest_due.is_zero(),
            "Expected 0 for interest due, got {}",
            previous_interest_due.amount()
        );
    } else {
        unreachable!()
    }
}

#[test]
fn full_repay() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease_address = super::open_lease(&mut test_case, downpayment, None);
    let borrowed_lpn = super::quote_borrow(&test_case, downpayment);
    let borrowed: PaymentCoin = price::total(borrowed_lpn, super::price_lpn_of().inv());
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, super::price_lpn_of()) + borrowed_lpn,
        super::price_lpn_of::<LeaseCurrency>().inv(),
    );

    repay(
        &mut test_case,
        lease_address.clone(),
        borrowed,
        lease_amount,
    );

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ super::price_lpn_of(),
        ),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = super::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn full_repay_with_max_ltd() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let percent = Percent::from_percent(10);
    let borrowed = percent.of(DOWNPAYMENT);
    let lease_address = super::open_lease(&mut test_case, downpayment, Some(percent));

    let lease_amount = (Percent::HUNDRED + percent).of(price::total(
        downpayment,
        Price::<PaymentCurrency, LeaseCurrency>::identity(),
    ));
    let expected_result = StateResponse::Opened {
        amount: lease_amount.into(),
        loan_interest_rate: Percent::from_permille(70),
        margin_interest_rate: Percent::from_permille(30),
        principal_due: price::total(percent.of(downpayment), super::price_lpn_of()).into(),
        previous_margin_due: LpnCoin::ZERO.into(),
        previous_interest_due: LpnCoin::ZERO.into(),
        current_margin_due: LpnCoin::ZERO.into(),
        current_interest_due: LpnCoin::ZERO.into(),
        validity: Timestamp::from_nanos(1537237454879305533),
        in_progress: None,
    };
    let query_result = super::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);

    repay(
        &mut test_case,
        lease_address.clone(),
        borrowed,
        lease_amount,
    );

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ super::price_lpn_of(),
        ),
        /* LPN -> Lease */ super::price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = super::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn full_repay_with_excess() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease_address = super::open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        super::quote_borrow(&test_case, downpayment),
        super::price_lpn_of().inv(),
    );
    let lease_amount = price::total(
        downpayment + borrowed,
        Price::<PaymentCurrency, LeaseCurrency>::identity(),
    );

    let overpayment = super::create_payment_coin(5);
    let payment: PaymentCoin = borrowed + overpayment;

    repay(&mut test_case, lease_address.clone(), payment, lease_amount);

    let query_result = super::state_query(&test_case, lease_address.as_str());

    assert_eq!(
        test_case
            .app
            .query()
            .query_all_balances(lease_address)
            .unwrap(),
        &[cwcoin::<Lpn, Amount>(overpayment.into())],
    );

    assert_eq!(
        test_case.app.query().query_all_balances("ica0").unwrap(),
        &[cwcoin::<LeaseCurrency, _>(price::total(
            price::total(downpayment + borrowed, lease::price_lpn_of()),
            lease::price_lpn_of().inv(),
        ))],
    );

    assert_eq!(
        query_result,
        StateResponse::Paid {
            amount: LeaseCoin::into(price::total(
                price::total(downpayment + borrowed, lease::price_lpn_of()),
                lease::price_lpn_of().inv(),
            )),
            in_progress: None,
        }
    );
}

pub(crate) fn repay<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    payment: PaymentCoin,
    lease_amount: LeaseCoin,
) -> AppResponse {
    let cw_payment: CwCoin = cwcoin(payment);

    let response: ResponseWithInterChainMsgs<'_, ()> =
        send_payment_and_transfer(test_case, contract_addr.clone(), cw_payment.clone());

    expect_swap(response);

    let swap_out_lpn: LpnCoin = price::total(payment, super::price_lpn_of());

    let response: ResponseWithInterChainMsgs<'_, ()> =
        do_swap(test_case, contract_addr.clone(), &cw_payment, swap_out_lpn);

    dex::expect_init_transfer_in(response);
    dex::do_transfer_in(test_case, contract_addr, swap_out_lpn, Some(lease_amount))
}

fn send_payment_and_transfer<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    cw_payment: CwCoin,
) -> ResponseWithInterChainMsgs<'_, ()> {
    let mut response: ResponseWithInterChainMsgs<'_, ()> = test_case
        .app
        .execute(
            Addr::unchecked(USER),
            contract_addr.clone(),
            &ExecuteMsg::Repay {},
            std::slice::from_ref(&cw_payment),
        )
        .unwrap()
        .ignore_response();

    response.expect_ibc_transfer(
        "channel-0",
        cw_payment.clone(),
        contract_addr.as_str(),
        "ica0",
    );

    () = response.unwrap_response();

    test_case
        .app
        .send_tokens(
            contract_addr.clone(),
            Addr::unchecked("ica0"),
            &[cw_payment],
        )
        .unwrap();

    test_case
        .app
        .sudo(contract_addr, &super::construct_response(Binary::default()))
        .unwrap()
        .ignore_response()
}

fn expect_swap(mut response: ResponseWithInterChainMsgs<'_, ()>) {
    response.expect_submit_tx(TestCase::LEASER_CONNECTION_ID, "0", 1);

    response.unwrap_response()
}

fn do_swap<'r, Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle>(
    test_case: &'r mut TestCase<Dispatcher, Treasury, Profit, Leaser, Lpp, Oracle, Addr>,
    contract_addr: Addr,
    cw_payment: &CwCoin,
    swap_out_lpn: LpnCoin,
) -> ResponseWithInterChainMsgs<'r, ()> {
    test_case
        .app
        .send_tokens(
            Addr::unchecked("ica0"),
            Addr::unchecked(ADMIN),
            std::slice::from_ref(cw_payment),
        )
        .unwrap();

    test_case.send_funds_from_admin(Addr::unchecked("ica0"), &[cwcoin(swap_out_lpn)]);

    test_case
        .app
        .sudo(
            contract_addr,
            &super::construct_response(Binary(platform::trx::encode_msg_responses(
                [swap::trx::build_exact_amount_in_resp(swap_out_lpn.into())].into_iter(),
            ))),
        )
        .unwrap()
        .ignore_response()
}
