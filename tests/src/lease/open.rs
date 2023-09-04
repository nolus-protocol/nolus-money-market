use finance::{coin::Coin, zero::Zero};

use crate::lease::heal;

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
    let downpayment = LeaseCoin::new(100);
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, &lease.clone().into_string());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    heal::heal_no_inconsistency(&mut test_case, lease);
}

#[test]
fn open_downpayment_different_than_lease_currency() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let downpayment = DOWNPAYMENT;
    let lease = super::open_lease(&mut test_case, downpayment, None);

    let query_result = super::state_query(&test_case, &lease.clone().into_string());
    let expected_result =
        super::expected_newly_opened_state(&test_case, downpayment, super::create_payment_coin(0));
    assert_eq!(query_result, expected_result);

    heal::heal_no_inconsistency(&mut test_case, lease);
}
