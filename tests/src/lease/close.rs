use finance::{price, zero::Zero};
use sdk::cosmwasm_std::Addr;

use super::{
    DOWNPAYMENT, LeaseCurrency, LeaseTestCase, LpnCoin, PaymentCoin, PaymentCurrency, repay,
};

#[test]
fn close_with_full_repay() {
    let mut test_case: LeaseTestCase = super::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = DOWNPAYMENT;
    let lease: Addr = super::open_lease(&mut test_case, downpayment, None);

    let borrowed_lpn: LpnCoin = super::quote_borrow(&test_case, downpayment);
    let borrowed =
        price::total(borrowed_lpn, super::price_lpn_of::<PaymentCurrency>().inv()).unwrap();
    let lease_amount = price::total(
        price::total(downpayment, super::price_lpn_of()).unwrap() + borrowed_lpn,
        super::price_lpn_of::<LeaseCurrency>().inv(),
    )
    .unwrap();

    let _app_response = repay::repay_full(
        &mut test_case,
        lease.clone(),
        borrowed,
        lease_amount,
        LpnCoin::ZERO,
    );
}
