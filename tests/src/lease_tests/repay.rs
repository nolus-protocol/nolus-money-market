use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::Fraction,
    percent::Percent,
    price::{self, Price},
    zero::Zero,
};
use lease::api::StateResponse;
use sdk::cosmwasm_std::Timestamp;

use crate::common::{cwcoin, leaser::Instantiator as LeaserInstantiator};

use super::{
    helpers, LeaseCoin, LeaseCurrency, Lpn, LpnCoin, PaymentCoin, PaymentCurrency, DOWNPAYMENT,
};

#[test]
fn partial_repay() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment = helpers::create_payment_coin(DOWNPAYMENT);

    let quote_result = helpers::quote_query(&test_case, downpayment);
    let amount: LpnCoin = quote_result.borrow.try_into().unwrap();
    let partial_payment = helpers::create_payment_coin(u128::from(amount) / 2);
    let expected_result =
        helpers::expected_newly_opened_state(&test_case, downpayment, partial_payment);

    let lease_address = helpers::open_lease(&mut test_case, downpayment, None);
    helpers::repay(&mut test_case, lease_address.clone(), partial_payment);

    let query_result = helpers::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn partial_repay_after_time() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = helpers::create_payment_coin(DOWNPAYMENT);

    let lease_address = helpers::open_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Duration::from_nanos(
        LeaserInstantiator::REPAYMENT_PERIOD.nanos() >> 1,
    ));

    let query_result = helpers::state_query(&test_case, lease_address.as_ref());

    let StateResponse::Opened {
        previous_margin_due,
        previous_interest_due,
        current_margin_due,
        ..
    } = query_result else {
        unreachable!()
    };

    helpers::feed_price(&mut test_case);

    let current_margin_to_pay: LpnCoin = LpnCoin::try_from(current_margin_due)
        .unwrap()
        .checked_div(2)
        .unwrap();

    helpers::repay(
        &mut test_case,
        lease_address.clone(),
        price::total(
            LpnCoin::try_from(previous_margin_due).unwrap()
                + LpnCoin::try_from(previous_interest_due).unwrap()
                + current_margin_to_pay,
            helpers::price_lpn_of::<PaymentCurrency>().inv(),
        ),
    );

    let query_result = helpers::state_query(&test_case, lease_address.as_str());

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
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = helpers::create_payment_coin(DOWNPAYMENT);
    let lease_address = helpers::open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        helpers::quote_borrow(&test_case, downpayment),
        helpers::price_lpn_of().inv(),
    );

    helpers::repay(&mut test_case, lease_address.clone(), borrowed);

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ helpers::price_lpn_of(),
        ),
        /* LPN -> Lease */ helpers::price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = helpers::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn full_repay_with_max_ltd() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment = helpers::create_payment_coin(DOWNPAYMENT);
    let percent = Percent::from_percent(10);
    let borrowed = Coin::new(percent.of(DOWNPAYMENT));
    let lease_address = helpers::open_lease(&mut test_case, downpayment, Some(percent));

    let expected_result = StateResponse::Opened {
        amount: (Percent::HUNDRED + percent)
            .of(price::total(
                downpayment,
                Price::<PaymentCurrency, LeaseCurrency>::identity(),
            ))
            .into(),
        loan_interest_rate: Percent::from_permille(70),
        margin_interest_rate: Percent::from_permille(30),
        principal_due: price::total(percent.of(downpayment), helpers::price_lpn_of()).into(),
        previous_margin_due: LpnCoin::ZERO.into(),
        previous_interest_due: LpnCoin::ZERO.into(),
        current_margin_due: LpnCoin::ZERO.into(),
        current_interest_due: LpnCoin::ZERO.into(),
        validity: Timestamp::from_nanos(1537237454879305533),
        in_progress: None,
    };
    let query_result = helpers::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);

    helpers::repay(&mut test_case, lease_address.clone(), borrowed);

    let expected_amount: LeaseCoin = price::total(
        price::total(
            downpayment + borrowed,
            /* Payment -> LPN */ helpers::price_lpn_of(),
        ),
        /* LPN -> Lease */ helpers::price_lpn_of().inv(),
    );
    let expected_result = StateResponse::Paid {
        amount: LeaseCoin::into(expected_amount),
        in_progress: None,
    };
    let query_result = helpers::state_query(&test_case, lease_address.as_str());

    assert_eq!(query_result, expected_result);
}

#[test]
fn full_repay_with_excess() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = helpers::create_payment_coin(DOWNPAYMENT);
    let lease_address = helpers::open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        helpers::quote_borrow(&test_case, downpayment),
        /* LPN -> Payment */ helpers::price_lpn_of().inv(),
    );

    let overpayment = helpers::create_payment_coin(5);
    let payment: PaymentCoin = borrowed + overpayment;

    helpers::repay(&mut test_case, lease_address.clone(), payment);

    let query_result = helpers::state_query(&test_case, lease_address.as_str());

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
            price::total(downpayment + borrowed, helpers::price_lpn_of()),
            helpers::price_lpn_of().inv(),
        ))],
    );

    assert_eq!(
        query_result,
        StateResponse::Paid {
            amount: LeaseCoin::into(price::total(
                price::total(downpayment + borrowed, helpers::price_lpn_of()),
                helpers::price_lpn_of().inv(),
            )),
            in_progress: None,
        }
    );
}
