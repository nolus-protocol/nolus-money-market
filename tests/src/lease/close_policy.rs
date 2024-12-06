use finance::coin::Coin;
use sdk::testing;

use crate::common::{oracle, ADMIN};

use super::{LeaseCurrency, LpnCurrency, PaymentCurrency, DOWNPAYMENT};

#[test]
fn tp_zero() {
    let mut test_case = super::create_test_case::<PaymentCurrency>();
    let _lease = super::open_lease(&mut test_case, DOWNPAYMENT, None);

    oracle::feed_price(
        &mut test_case,
        testing::user(ADMIN),
        Coin::<LeaseCurrency>::from(1),
        Coin::<LpnCurrency>::from(45),
    );

    // let response: AppResponse =
    //     deliver_new_price(&mut test_case, lease, base, quote).unwrap_response();
}
