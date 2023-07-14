use finance::price;
use lease::api::StateResponse;
use sdk::cosmwasm_std::Addr;

use crate::common::{cwcoin, USER};

use super::{helpers, LeaseCoin, LeaseCurrency, PaymentCoin, PaymentCurrency, DOWNPAYMENT};

#[test]
fn state_closed() {
    let mut test_case = helpers::create_test_case::<PaymentCurrency>();
    let downpayment: PaymentCoin = helpers::create_payment_coin(DOWNPAYMENT);
    let lease_address = helpers::open_lease(&mut test_case, downpayment, None);
    let borrowed: PaymentCoin = price::total(
        helpers::quote_borrow(&test_case, downpayment),
        helpers::price_lpn_of::<PaymentCurrency>().inv(),
    );
    let lease_amount: LeaseCoin = price::total(
        price::total(downpayment, helpers::price_lpn_of())
            + helpers::quote_borrow(&test_case, downpayment),
        helpers::price_lpn_of::<LeaseCurrency>().inv(),
    );
    helpers::repay(&mut test_case, lease_address.clone(), borrowed);

    let user_balance: LeaseCoin =
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap();

    helpers::close(
        &mut test_case,
        lease_address.clone(),
        &[cwcoin(lease_amount)],
    );

    let query_result = helpers::state_query(&test_case, lease_address.as_str());
    let expected_result = StateResponse::Closed();

    assert_eq!(query_result, expected_result);

    assert_eq!(
        platform::bank::balance(&Addr::unchecked(USER), &test_case.app.query()).unwrap(),
        user_balance + lease_amount
    );
}
