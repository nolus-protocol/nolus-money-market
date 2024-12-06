use ::lease::api::position::ChangeCmd;
use finance::{coin::Coin, percent::Percent};
use sdk::testing;

use crate::{
    common::{oracle, ADMIN},
    lease::{self, LeaseCurrency, LeaserInstantiator, LpnCurrency, PaymentCurrency, DOWNPAYMENT},
};

#[test]
fn trigger() {
    let mut test_case = lease::create_test_case::<PaymentCurrency>();
    // LeaseC/LpnC = 1
    let lease = lease::open_lease(&mut test_case, DOWNPAYMENT, None);

    let tp = LeaserInstantiator::INITIAL_LTV;
    let sl = LeaserInstantiator::INITIAL_LTV + Percent::from_permille(1);
    super::change_ok(
        &mut test_case,
        lease.clone(),
        Some(ChangeCmd::Set(tp)),
        Some(ChangeCmd::Set(sl)),
    );

    // LeaseC/LpnC = 0.999
    oracle::feed_price(
        &mut test_case,
        testing::user(ADMIN),
        Coin::<LeaseCurrency>::from(999),
        Coin::<LpnCurrency>::from(1000),
    );
}
