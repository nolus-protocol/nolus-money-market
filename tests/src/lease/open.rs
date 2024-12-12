use finance::{coin::Coin, zero::Zero};

use crate::{common::leaser::Instantiator, lease::heal};

use super::{LeaseCoin, LeaseCurrency, PaymentCurrency, DOWNPAYMENT};

#[test]
#[should_panic = "[Lease] No payment sent"]
fn open_zero_downpayment() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = Coin::<PaymentCurrency>::ZERO;
    super::try_init_lease(&mut test_case, downpayment, None);
}

#[test]
fn open_downpayment_lease_currency() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(10_000);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(expected_result, query_result);

    heal::heal_no_inconsistency(&mut test_case.app, lease);
}

#[test]
fn open_downpayment_different_than_lease_currency() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, lease.clone());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    heal::heal_no_inconsistency(&mut test_case.app, lease);
}

#[test]
fn open_takes_longer() {
    let mut test_case = super::create_test_case::<LeaseCurrency>();
    let downpayment = LeaseCoin::new(100);
    let lease = super::try_init_lease(&mut test_case, downpayment, None);

    test_case.app.time_shift(Instantiator::REPAYMENT_PERIOD);
    super::feed_price(&mut test_case);

    super::complete_init_lease(&mut test_case, downpayment, None, &lease);

    heal::heal_no_inconsistency(&mut test_case.app, lease);
}
