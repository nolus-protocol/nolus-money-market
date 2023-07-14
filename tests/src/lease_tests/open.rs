use super::{helpers, LeaseCoin, LeaseCurrency, PaymentCurrency, DOWNPAYMENT};

#[test]
#[should_panic = "[Lease] No payment sent"]
fn open_zero_downpayment() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment = helpers::create_payment_coin(0);
    helpers::try_init_lease(&mut test_case, downpayment, None);
}

#[test]
fn open_downpayment_lease_currency() {
    let mut test_case = helpers::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(100);
    let lease = helpers::open_lease(&mut test_case, downpayment, None);

    let query_result = helpers::state_query(&test_case, &lease.into_string());
    let expected_result = helpers::expected_newly_opened_state(
        &test_case,
        downpayment,
        helpers::create_payment_coin(0),
    );
    assert_eq!(query_result, expected_result);
}

#[test]
fn open_downpayment_different_than_lease_currency() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment = helpers::create_payment_coin(DOWNPAYMENT);
    let lease = helpers::open_lease(&mut test_case, downpayment, None);

    let query_result = helpers::state_query(&test_case, &lease.into_string());
    let expected_result = helpers::expected_newly_opened_state(
        &test_case,
        downpayment,
        helpers::create_payment_coin(0),
    );
    assert_eq!(query_result, expected_result);
}
