use currencies::{Lpn, testing::PaymentC3};
use finance::{
    coin::{Amount, Coin},
    duration::Duration,
    fraction::FractionLegacy,
    liability::Liability,
    percent::Percent100,
    price::{self, Price},
};

use crate::{
    finance::LpnCoin,
    lease::tests,
    position::{DueTrait, OverdueCollection, close::Policy as ClosePolicy},
};

use super::Spec;

const HALF_STEP: Percent100 = Percent100::from_permille(5);
const STEP: Percent100 = Percent100::from_permille(5 + 5);
const MAX_DEBT: Percent100 = Percent100::from_permille(800);
const RECALC_IN: Duration = Duration::from_hours(1);

type TestCurrency = PaymentC3;
type TestLpn = Lpn;

struct TestDue {
    total_due: LpnCoin,
    overdue: LpnCoin,
}
impl DueTrait for TestDue {
    fn total_due(&self) -> LpnCoin {
        self.total_due
    }

    #[track_caller]
    fn overdue_collection(&self, min_amount: LpnCoin) -> OverdueCollection {
        if self.overdue.is_zero() || self.overdue < min_amount {
            OverdueCollection::StartIn(Duration::from_days(5))
        } else {
            OverdueCollection::Overdue(self.overdue)
        }
    }
}

const fn lease_coin(amount: Amount) -> Coin<TestCurrency> {
    Coin::new(amount)
}

fn due(total_due: Amount, overdue_collectable: Amount) -> TestDue {
    TestDue {
        total_due: tests::lpn_coin(total_due),
        overdue: tests::lpn_coin(overdue_collectable),
    }
}

fn spec(min_asset: Amount, min_transaction: Amount) -> Spec {
    let liability = Liability::new(
        Percent100::from_percent(65),
        Percent100::from_percent(70),
        Percent100::from_percent(73),
        Percent100::from_percent(75),
        Percent100::from_percent(78),
        MAX_DEBT,
        Duration::from_hours(1),
    );
    Spec::new(
        liability,
        ClosePolicy::default(),
        tests::lpn_coin(min_asset),
        tests::lpn_coin(min_transaction),
    )
}

fn ltv_to_price(
    asset: Coin<TestCurrency>,
    due: LpnCoin,
) -> impl FnMut(Percent100) -> Price<TestCurrency, TestLpn> {
    move |ltv| price(ltv.of(asset), due)
}

fn price(
    price_asset: Coin<TestCurrency>,
    price_lpn: Coin<TestLpn>,
) -> Price<TestCurrency, TestLpn> {
    price::total_of(price_asset).is(price_lpn)
}

fn price_from_amount(price_asset: Amount, price_lpn: Amount) -> Price<TestCurrency, TestLpn> {
    price(lease_coin(price_asset), tests::lpn_coin(price_lpn))
}

fn spec_with_first(warn: Percent100, min_asset: Amount, min_transaction: Amount) -> Spec {
    spec_with_max(warn + STEP + STEP + STEP, min_asset, min_transaction)
}

fn spec_with_second(warn: Percent100, min_asset: Amount, min_transaction: Amount) -> Spec {
    spec_with_max(warn + STEP + STEP, min_asset, min_transaction)
}

fn spec_with_third(warn: Percent100, min_asset: Amount, min_transaction: Amount) -> Spec {
    spec_with_max(warn + STEP, min_asset, min_transaction)
}

// init = 1%, healthy = 1%, first = max - 3, second = max - 2, third = max - 1
fn spec_with_max(max: Percent100, min_asset: Amount, min_transaction: Amount) -> Spec {
    let initial = STEP;
    assert!(initial < max - STEP - STEP - STEP);

    let healthy = initial + Percent100::ZERO;
    let third_liquidity_warning = max - STEP;
    let second_liquidity_warning = third_liquidity_warning - STEP;
    let first_liquidity_warning = second_liquidity_warning - STEP;

    let liability = Liability::new(
        initial,
        healthy,
        first_liquidity_warning,
        second_liquidity_warning,
        third_liquidity_warning,
        max,
        RECALC_IN,
    );
    Spec::new(
        liability,
        ClosePolicy::default(),
        tests::lpn_coin(min_asset),
        tests::lpn_coin(min_transaction),
    )
}

mod test_calc_borrow {
    use finance::percent::Percent;

    use crate::{lease::tests, position::PositionError};

    #[test]
    fn downpayment_less_than_min() {
        let spec = super::spec(560, 300);

        let downpayment_less = spec.calc_borrow_amount(tests::lpn_coin(299), None);
        assert!(matches!(
            downpayment_less,
            Err(PositionError::InsufficientTransactionAmount(_))
        ));

        let borrow = spec.calc_borrow_amount(tests::lpn_coin(300), None);
        assert_eq!(tests::lpn_coin(557), borrow.unwrap());
    }

    #[test]
    fn borrow_less_than_min() {
        let spec = super::spec(600, 300);

        let borrow_less =
            spec.calc_borrow_amount(tests::lpn_coin(300), Some(Percent::from_percent(99)));
        assert!(matches!(
            borrow_less,
            Err(PositionError::InsufficientTransactionAmount(_))
        ));

        let borrow =
            spec.calc_borrow_amount(tests::lpn_coin(300), Some(Percent::from_percent(100)));
        assert_eq!(tests::lpn_coin(300), borrow.unwrap());
    }

    #[test]
    fn lease_less_than_min() {
        let spec = super::spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(tests::lpn_coin(349), None);
        assert!(matches!(
            borrow_1,
            Err(PositionError::InsufficientAssetAmount(_))
        ));

        let borrow_2 = spec.calc_borrow_amount(tests::lpn_coin(350), None);
        assert_eq!(tests::lpn_coin(650), borrow_2.unwrap());

        let borrow_3 =
            spec.calc_borrow_amount(tests::lpn_coin(550), Some(Percent::from_percent(81)));
        assert!(matches!(
            borrow_3,
            Err(PositionError::InsufficientAssetAmount(_))
        ));

        let borrow_3 =
            spec.calc_borrow_amount(tests::lpn_coin(550), Some(Percent::from_percent(82)));
        assert_eq!(tests::lpn_coin(451), borrow_3.unwrap());
    }

    #[test]
    fn valid_borrow_amount() {
        let spec = super::spec(1_000, 300);

        let borrow_1 = spec.calc_borrow_amount(tests::lpn_coin(540), None);
        assert_eq!(tests::lpn_coin(1002), borrow_1.unwrap());

        let borrow_2 =
            spec.calc_borrow_amount(tests::lpn_coin(870), Some(Percent::from_percent(100)));
        assert_eq!(tests::lpn_coin(870), borrow_2.unwrap());

        let borrow_3 =
            spec.calc_borrow_amount(tests::lpn_coin(650), Some(Percent::from_percent(150)));
        assert_eq!(tests::lpn_coin(975), borrow_3.unwrap());
    }
}

mod test_debt {

    use finance::{liability::Zone, percent::Percent100};

    use crate::position::{Cause, Debt, spec::test::STEP};

    #[test]
    fn no_debt() {
        let warn_ltv = Percent100::from_permille(11);
        let spec = super::spec_with_first(warn_ltv, 1, 1);
        let asset = super::lease_coin(100);

        assert_eq!(
            spec.debt(asset, &super::due(0, 0), super::price_from_amount(1, 1)),
            Debt::No,
        );
        assert_eq!(
            spec.debt(asset, &super::due(0, 0), super::price_from_amount(3, 1)),
            Debt::No,
        );
    }

    #[test]
    fn warnings_none_zero_liq() {
        let warn_ltv = Percent100::from_percent(51);
        let spec = super::spec_with_first(warn_ltv, 1, 1);
        let asset = super::lease_coin(100);

        assert_eq!(
            spec.debt(asset, &super::due(1, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                // steadiness: steady_to(warn_ltv, asset, 1)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(1, 0), super::price_from_amount(5, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 1)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(50, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 50)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(25, 0), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 25)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(51, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 51)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(17, 0), super::price_from_amount(3, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 17)
            },
        );
    }

    #[test]
    fn warnings_none_min_transaction() {
        let warn_ltv = Percent100::from_percent(51);
        let spec = super::spec_with_first(warn_ltv, 1, 15);
        let asset = super::lease_coin(100);

        assert_eq!(
            spec.debt(asset, &super::due(50, 14), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 50)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(25, 4), super::price_from_amount(2, 3)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 25)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(51, 14), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 51)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(17, 4), super::price_from_amount(3, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 17)
            },
        );
    }

    #[test]
    fn warnings_first() {
        let warn_ltv = Percent100::from_permille(712);
        let spec = super::spec_with_first(warn_ltv, 10, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(711, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 711)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(237, 0), super::price_from_amount(3, 1)),
            Debt::Ok {
                zone: Zone::no_warnings(warn_ltv),
                //steaduness: steady_to(warn_ltv, asset, 237)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(712, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 712)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(178, 0), super::price_from_amount(4, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 178)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(712, 1), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(1), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(89, 1), super::price_from_amount(8, 1)),
            Debt::partial(super::lease_coin(8), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(712, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 712)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(103, 0), super::price_from_amount(7, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 103)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(722, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv + STEP, warn_ltv + STEP + STEP),
                //steaduness: steady_in(warn_ltv + STEP, warn_ltv + STEP + STEP, asset, 722)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(361, 0), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv + STEP, warn_ltv + STEP + STEP),
                //steaduness: steady_in(warn_ltv + STEP, warn_ltv + STEP + STEP, asset, 361)
            },
        );
    }

    #[test]
    fn warnings_first_min_transaction() {
        let warn_ltv = Percent100::from_permille(712);
        let spec = super::spec_with_first(warn_ltv, 10, 3);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(712, 2), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 712)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(356, 1), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 356)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(721, 2), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 721)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(721, 5), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(5), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(240, 3), super::price_from_amount(3, 1)),
            Debt::partial(super::lease_coin(9), Cause::Overdue()),
        );
    }

    #[test]
    fn warnings_second() {
        let warn_ltv = Percent100::from_permille(123);
        let spec = super::spec_with_second(warn_ltv, 10, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(122, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv - STEP, warn_ltv),
                //steaduness: steady_in(warn_ltv - STEP, warn_ltv, asset, 122)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(15, 0), super::price_from_amount(8, 1)),
            Debt::Ok {
                zone: Zone::first(warn_ltv - STEP, warn_ltv),
                //steaduness: steady_in(warn_ltv - STEP, warn_ltv, asset, 15)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(123, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 123)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(82, 0), super::price_from_amount(3, 2)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 82)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(123, 4), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(4), Cause::Overdue())
        );
        assert_eq!(
            spec.debt(asset, &super::due(132, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 132)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(66, 0), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 66)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(133, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_ltv + STEP, warn_ltv + STEP + STEP),
                //steaduness: steady_in(warn_ltv + STEP, warn_ltv + STEP + STEP, asset, 133)
            },
        );
    }

    #[test]
    fn warnings_second_min_transaction() {
        let warn_ltv = Percent100::from_permille(123);
        let spec = super::spec_with_second(warn_ltv, 10, 5);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(128, 4), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 128)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(32, 1), super::price_from_amount(4, 1)),
            Debt::Ok {
                zone: Zone::second(warn_ltv, warn_ltv + STEP),
                //steaduness: steady_in(warn_ltv, warn_ltv + STEP, asset, 32)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(128, 5), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(5), Cause::Overdue())
        );
    }

    #[test]
    fn warnings_third() {
        let warn_third_ltv = Percent100::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = super::spec_with_third(warn_third_ltv, 100, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(380, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                //steaduness: steady_in(warn_third_ltv - STEP, warn_third_ltv, asset, 380)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(190, 0), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                //steaduness: steady_in(warn_third_ltv - STEP, warn_third_ltv, asset, 190)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(381, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 381)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(381, 375), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(375), Cause::Overdue())
        );
        assert_eq!(
            spec.debt(asset, &super::due(573, 562), super::price_from_amount(2, 3)),
            Debt::partial(super::lease_coin(374), Cause::Overdue())
        );
        assert_eq!(
            spec.debt(asset, &super::due(390, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 390)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(391, 0), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(384),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn warnings_third_min_transaction() {
        let warn_third_ltv = Percent100::from_permille(381);
        let max_ltv = warn_third_ltv + STEP;
        let spec = super::spec_with_third(warn_third_ltv, 100, 386);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(380, 1), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                //steaduness: steady_in(warn_third_ltv - STEP, warn_third_ltv, asset, 380)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(126, 1), super::price_from_amount(3, 1)),
            Debt::Ok {
                zone: Zone::second(warn_third_ltv - STEP, warn_third_ltv),
                //steaduness: steady_in(warn_third_ltv - STEP, warn_third_ltv, asset, 126)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(381, 375), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 381)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(391, 385), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 391)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(391, 386), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(386), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(392, 0), super::price_from_amount(1, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 392)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(364, 0), super::price_from_amount(2, 1)),
            Debt::Ok {
                zone: Zone::third(warn_third_ltv, max_ltv),
                //steaduness: steady_in(warn_third_ltv, max_ltv, asset, 364)
            },
        );
        assert_eq!(
            spec.debt(asset, &super::due(393, 0), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(386),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(788, 0), super::price_from_amount(1, 2)),
            Debt::partial(
                super::lease_coin(387),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
    }

    #[test]
    fn liquidate_partial() {
        let max_ltv = Percent100::from_permille(881);
        let spec = super::spec_with_max(max_ltv, 100, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(880, 1), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(1), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(139, 1), super::price_from_amount(4, 1)),
            Debt::partial(super::lease_coin(4), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(881, 879), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(879),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(881, 880), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(880), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(294, 294), super::price_from_amount(1, 3)),
            Debt::partial(super::lease_coin(98), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(294, 293), super::price_from_amount(3, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            spec.debt(asset, &super::due(1000, 1), super::price_from_amount(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_partial_min_asset() {
        let max_ltv = Percent100::from_permille(881);
        let spec = super::spec_with_max(max_ltv, 100, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(900, 897), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(898),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(900, 899), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(899), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(233, 233), super::price_from_amount(3, 1)),
            Debt::partial(super::lease_coin(699), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(901, 889), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(900),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(902, 889), super::price_from_amount(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full() {
        let max_ltv = Percent100::from_permille(768);
        let spec = super::spec_with_max(max_ltv, 230, 1);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(768, 765), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(765),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(
                asset,
                &super::due(1560, 1552),
                super::price_from_amount(1, 2)
            ),
            Debt::partial(
                super::lease_coin(777),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(768, 768), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(768), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(
                asset,
                &super::due(1560, 1556),
                super::price_from_amount(1, 2)
            ),
            Debt::partial(super::lease_coin(778), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(788, 768), super::price_from_amount(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_liability() {
        let max_ltv = Percent100::from_permille(673);
        let spec = super::spec_with_max(max_ltv, 120, 15);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(882, 1), super::price_from_amount(1, 1)),
            Debt::partial(
                super::lease_coin(880),
                Cause::Liability {
                    ltv: max_ltv,
                    healthy_ltv: STEP
                }
            ),
        );
        assert_eq!(
            spec.debt(asset, &super::due(883, 1), super::price_from_amount(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            spec.debt(asset, &super::due(294, 1), super::price_from_amount(3, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
        assert_eq!(
            spec.debt(asset, &super::due(1000, 1), super::price_from_amount(1, 1)),
            Debt::full(Cause::Liability {
                ltv: max_ltv,
                healthy_ltv: STEP
            }),
        );
    }

    #[test]
    fn liquidate_full_overdue() {
        let max_ltv = Percent100::from_permille(773);
        let spec = super::spec_with_max(max_ltv, 326, 15);
        let asset = super::lease_coin(1000);

        assert_eq!(
            spec.debt(asset, &super::due(772, 674), super::price_from_amount(1, 1)),
            Debt::partial(super::lease_coin(674), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(
                asset,
                &super::due(1674, 1674),
                super::price_from_amount(1, 2)
            ),
            Debt::partial(super::lease_coin(837), Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(asset, &super::due(772, 675), super::price_from_amount(1, 1)),
            Debt::full(Cause::Overdue()),
        );
        assert_eq!(
            spec.debt(
                asset,
                &super::due(1676, 1676),
                super::price_from_amount(1, 2)
            ),
            Debt::full(Cause::Overdue()),
        );
    }
}

mod test_steadiness {

    use finance::{
        coin::Coin, fraction::FractionLegacy, percent::Percent100, range::RightOpenRange,
    };

    use crate::{
        api::position::{ChangeCmd, ClosePolicyChange},
        finance::LpnCoin,
        position::{DueTrait, Steadiness, spec::test::STEP},
    };

    use super::{HALF_STEP, RECALC_IN, TestCurrency};

    const TP: Percent100 = Percent100::from_permille(490);
    const LTV: Percent100 = TP.checked_add(STEP).expect("should not exceed 100%");
    const WARN_LTV: Percent100 = LTV.checked_add(STEP).expect("should not exceed 100%");
    const SL: Percent100 = WARN_LTV.checked_add(STEP).expect("should not exceed 100%");

    const ASSET: Coin<TestCurrency> = super::lease_coin(1000);

    #[test]
    fn no_close() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);

        {
            //left-open zone
            assert_eq!(
                steady_to(WARN_LTV, ASSET, due.total_due()),
                spec.steadiness(ASSET, &due, super::price(LTV.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
                spec.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );
        }
        {
            //full zone
            assert_eq!(
                steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
                spec.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );

            assert_eq!(
                steady_in(
                    WARN_LTV + STEP,
                    WARN_LTV + STEP + STEP,
                    ASSET,
                    due.total_due()
                ),
                spec.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + STEP).of(ASSET), due.total_due())
                )
            );
        }
    }

    #[test]
    fn tp_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        {
            //left-open zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Set(TP)),
                        stop_loss: Some(ChangeCmd::Reset),
                    },
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due()),
                )
                .unwrap();
            assert_eq!(
                steady_in(TP, WARN_LTV, ASSET, due.total_due()),
                spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );
        }
        {
            //full zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Set(WARN_LTV + HALF_STEP)),
                        stop_loss: Some(ChangeCmd::Reset),
                    },
                    ASSET,
                    &due,
                    super::price((WARN_LTV + STEP).of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_in(
                    WARN_LTV + HALF_STEP,
                    WARN_LTV + STEP,
                    ASSET,
                    due.total_due()
                ),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due())
                )
            );

            assert_eq!(
                steady_in(
                    WARN_LTV + STEP,
                    WARN_LTV + STEP + STEP,
                    ASSET,
                    due.total_due()
                ),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + STEP).of(ASSET), due.total_due())
                )
            );
        }
    }

    #[test]
    fn sl_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        {
            //left open zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(TP)),
                    },
                    ASSET,
                    &due,
                    super::price((TP - STEP).of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_to(TP, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((TP - STEP).of(ASSET), due.total_due())
                )
            );
        }
        {
            //full zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(WARN_LTV + HALF_STEP)),
                    },
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_to(WARN_LTV, ASSET, due.total_due()),
                spec_tp.steadiness(ASSET, &due, super::price(LTV.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, WARN_LTV + HALF_STEP, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );
        }
    }

    #[test]
    fn sl_out() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        {
            //left open zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(SL)),
                    },
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_to(WARN_LTV, ASSET, due.total_due()),
                spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, SL, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );
        }
        {
            //full zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: None,
                        stop_loss: Some(ChangeCmd::Set(WARN_LTV + STEP + STEP)),
                    },
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_to(WARN_LTV, ASSET, due.total_due()),
                spec_tp.steadiness(ASSET, &due, super::price(LTV.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due())
                )
            );
        }
    }

    #[test]
    fn tp_in_sl_out() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        {
            //left_open zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Set(TP)),
                        stop_loss: Some(ChangeCmd::Set(SL)),
                    },
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_in(TP, WARN_LTV, ASSET, due.total_due()),
                spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
            );

            assert_eq!(
                steady_in(WARN_LTV, SL, ASSET, due.total_due()),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price(WARN_LTV.of(ASSET), due.total_due())
                )
            );
        }
        {
            //full zone
            let spec_tp = spec
                .change_close_policy(
                    ClosePolicyChange {
                        take_profit: Some(ChangeCmd::Set(WARN_LTV + HALF_STEP)),
                        stop_loss: Some(ChangeCmd::Set(WARN_LTV + STEP + HALF_STEP)),
                    },
                    ASSET,
                    &due,
                    super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due()),
                )
                .unwrap();

            assert_eq!(
                steady_in(
                    WARN_LTV + HALF_STEP,
                    WARN_LTV + STEP,
                    ASSET,
                    due.total_due()
                ),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due())
                )
            );

            assert_eq!(
                steady_in(
                    WARN_LTV + STEP,
                    WARN_LTV + STEP + HALF_STEP,
                    ASSET,
                    due.total_due()
                ),
                spec_tp.steadiness(
                    ASSET,
                    &due,
                    super::price((WARN_LTV + STEP).of(ASSET), due.total_due())
                )
            );
        }
    }

    #[test]
    fn tp_in_sl_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(WARN_LTV + HALF_STEP)),
                    stop_loss: Some(ChangeCmd::Set(WARN_LTV + STEP)),
                },
                ASSET,
                &due,
                super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_in(
                WARN_LTV + HALF_STEP,
                WARN_LTV + STEP,
                ASSET,
                due.total_due()
            ),
            spec_tp.steadiness(
                ASSET,
                &due,
                super::price((WARN_LTV + HALF_STEP).of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_no_close() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);

        assert_eq!(
            steady_to(WARN_LTV, ASSET, due.total_due()),
            spec.steadiness(ASSET, &due, super::price(LTV.of(ASSET), due.total_due()))
        );

        assert_eq!(
            steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
            spec.steadiness(
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_tp_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(TP)),
                    stop_loss: None,
                },
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_in(TP, WARN_LTV, ASSET, due.total_due()),
            spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
        );

        assert_eq!(
            steady_in(WARN_LTV, WARN_LTV + STEP, ASSET, due.total_due()),
            spec_tp.steadiness(
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_sl_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: None,
                    stop_loss: Some(ChangeCmd::Set(TP)),
                },
                ASSET,
                &due,
                super::price((TP - STEP).of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_to(TP, ASSET, due.total_due()),
            spec_tp.steadiness(
                ASSET,
                &due,
                super::price((TP - STEP).of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_sl_out() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: None,
                    stop_loss: Some(ChangeCmd::Set(SL)),
                },
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_to(WARN_LTV, ASSET, due.total_due()),
            spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
        );

        assert_eq!(
            steady_in(WARN_LTV, SL, ASSET, due.total_due()),
            spec_tp.steadiness(
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_tp_in_sl_out() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(TP)),
                    stop_loss: Some(ChangeCmd::Set(SL)),
                },
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_in(TP, WARN_LTV, ASSET, due.total_due()),
            spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
        );

        assert_eq!(
            steady_in(WARN_LTV, SL, ASSET, due.total_due()),
            spec_tp.steadiness(
                ASSET,
                &due,
                super::price(WARN_LTV.of(ASSET), due.total_due())
            )
        );
    }

    #[test]
    fn left_open_zone_tp_in_sl_in() {
        let spec = super::spec_with_first(WARN_LTV, 1, 1);
        let due = super::due(1, 0);
        let spec_tp = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(TP)),
                    stop_loss: Some(ChangeCmd::Set(LTV)),
                },
                ASSET,
                &due,
                super::price(TP.of(ASSET), due.total_due()),
            )
            .unwrap();

        assert_eq!(
            steady_in(TP, LTV, ASSET, due.total_due()),
            spec_tp.steadiness(ASSET, &due, super::price(TP.of(ASSET), due.total_due()))
        );
    }

    fn steady_to(
        ltv: Percent100,
        asset: Coin<TestCurrency>,
        due: LpnCoin,
    ) -> Steadiness<TestCurrency> {
        Steadiness::new(
            RECALC_IN,
            RightOpenRange::up_to(ltv).invert(super::ltv_to_price(asset, due)),
        )
    }

    fn steady_in(
        ltv_from: Percent100,
        ltv_to: Percent100,
        asset: Coin<TestCurrency>,
        due: LpnCoin,
    ) -> Steadiness<TestCurrency> {
        Steadiness::new(
            RECALC_IN,
            RightOpenRange::up_to(ltv_to)
                .cut_to(ltv_from)
                .invert(super::ltv_to_price(asset, due)),
        )
    }
}

mod test_validate_payment {
    use crate::position::PositionError;

    #[test]
    fn insufficient_payment() {
        let spec = super::spec(65, 16);
        let result_1 = spec.validate_payment(super::lease_coin(15), super::price_from_amount(1, 1));
        assert!(matches!(
            result_1,
            Err(PositionError::InsufficientTransactionAmount(_))
        ));
        let result_2 = spec.validate_payment(super::lease_coin(16), super::price_from_amount(1, 1));
        assert!(result_2.is_ok());

        let result_3 = spec.validate_payment(super::lease_coin(45), super::price_from_amount(3, 1));
        assert!(matches!(
            result_3,
            Err(PositionError::InsufficientTransactionAmount(_))
        ));
        let result_4 = spec.validate_payment(super::lease_coin(8), super::price_from_amount(1, 2));
        assert!(result_4.is_ok());
    }
}

mod test_validate_close {
    use crate::position::PositionError;

    #[test]
    fn too_small_amount() {
        let spec = super::spec(75, 15);
        let asset = super::lease_coin(100);

        let result_1 = spec.validate_close_amount(
            asset,
            super::lease_coin(14),
            super::price_from_amount(1, 1),
        );
        assert!(matches!(
            result_1,
            Err(PositionError::PositionCloseAmountTooSmall(_))
        ));

        let result_2 =
            spec.validate_close_amount(asset, super::lease_coin(6), super::price_from_amount(1, 2));
        assert!(matches!(
            result_2,
            Err(PositionError::PositionCloseAmountTooSmall(_))
        ));
    }

    #[test]
    fn amount_as_min_transaction() {
        let spec = super::spec(85, 15);
        let asset = super::lease_coin(100);

        let result_1 = spec.validate_close_amount(
            asset,
            super::lease_coin(15),
            super::price_from_amount(1, 1),
        );
        assert!(result_1.is_ok());

        let result_2 =
            spec.validate_close_amount(asset, super::lease_coin(5), super::price_from_amount(1, 3));
        assert!(result_2.is_ok());
    }

    #[test]
    fn too_big_amount() {
        let spec = super::spec(25, 1);
        let asset = super::lease_coin(100);

        let result_1 = spec.validate_close_amount(
            asset,
            super::lease_coin(76),
            super::price_from_amount(1, 1),
        );
        assert!(matches!(
            result_1,
            Err(PositionError::PositionCloseAmountTooBig(_))
        ));

        let result_2 = spec.validate_close_amount(
            asset,
            super::lease_coin(64),
            super::price_from_amount(3, 2),
        );
        assert!(matches!(
            result_2,
            Err(PositionError::PositionCloseAmountTooBig(_))
        ));
    }

    #[test]
    fn amount_as_min_asset() {
        let spec = super::spec(25, 1);
        let asset = super::lease_coin(100);

        let result_1 = spec.validate_close_amount(
            asset,
            super::lease_coin(75),
            super::price_from_amount(1, 1),
        );
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(
            asset,
            super::lease_coin(62),
            super::price_from_amount(3, 2),
        );
        assert!(result_2.is_ok());
    }

    #[test]
    fn valid_amount() {
        let spec = super::spec(40, 10);
        let asset = super::lease_coin(100);

        let result_1 = spec.validate_close_amount(
            asset,
            super::lease_coin(53),
            super::price_from_amount(1, 1),
        );
        assert!(result_1.is_ok());

        let result_2 = spec.validate_close_amount(
            asset,
            super::lease_coin(89),
            super::price_from_amount(1, 4),
        );
        assert!(result_2.is_ok());
    }
}

mod test_check_close {
    use finance::percent::Percent100;

    use crate::{
        api::position::{ChangeCmd, ClosePolicyChange},
        position::{CloseStrategy, PositionError},
    };

    #[test]
    fn set_higher_than_max() {
        let spec = super::spec(40, 10);

        let stop_loss_trigger = super::MAX_DEBT;
        assert_eq!(
            Err(PositionError::liquidation_conflict(
                super::MAX_DEBT,
                CloseStrategy::StopLoss(super::MAX_DEBT)
            )),
            spec.change_close_policy(
                ClosePolicyChange {
                    take_profit: None,
                    stop_loss: Some(ChangeCmd::Set(stop_loss_trigger)),
                },
                super::lease_coin(1000),
                &super::due(550, 0),
                super::price_from_amount(1, 2),
            )
        );
    }

    #[test]
    fn set_reset_stop_loss() {
        let spec = super::spec(40, 10);
        let asset = super::lease_coin(100);

        assert_eq!(
            None,
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );

        let stop_loss_trigger = Percent100::from_percent(46);
        assert_eq!(
            Err(PositionError::trigger_close(
                Percent100::from_ratio(super::lease_coin(920), super::lease_coin(1000 * 2)),
                CloseStrategy::StopLoss(stop_loss_trigger)
            )),
            spec.change_close_policy(
                ClosePolicyChange {
                    take_profit: None,
                    stop_loss: Some(ChangeCmd::Set(stop_loss_trigger)),
                },
                super::lease_coin(1000),
                &super::due(920, 0),
                super::price_from_amount(1, 2),
            )
        );
        let spec = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: None,
                    stop_loss: Some(ChangeCmd::Set(stop_loss_trigger)),
                },
                super::lease_coin(1000),
                &super::due(550, 0),
                super::price_from_amount(1, 2),
            )
            .unwrap();

        assert_eq!(
            None,
            // 90 LPNs due = 45 Asset units due, 45/100 = 45% LPN
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );

        assert_eq!(
            Some(CloseStrategy::StopLoss(stop_loss_trigger)),
            // 92 LPNs due = 46 Asset units due, 46/100 = 46% LPN
            spec.check_close(asset, &super::due(92, 0), super::price_from_amount(1, 2))
        );

        let spec = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(stop_loss_trigger)),
                    stop_loss: Some(ChangeCmd::Reset),
                },
                super::lease_coin(1000),
                &super::due(920, 0),
                super::price_from_amount(1, 2),
            )
            .unwrap();

        assert_eq!(
            None,
            // 92 LPNs due = 46 Asset units due, 46/100 = 46% LPN
            spec.check_close(asset, &super::due(92, 0), super::price_from_amount(1, 2))
        );
        assert_eq!(
            Some(CloseStrategy::TakeProfit(stop_loss_trigger)),
            // 90 LPNs due = 45 Asset units due, 45/100 = 45% LPN
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );
    }

    #[test]
    fn set_reset_take_profit() {
        let spec = super::spec(40, 10);
        let asset = super::lease_coin(100);

        assert_eq!(
            None,
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );

        let take_profit_trigger = Percent100::from_percent(46);
        assert_eq!(
            Err(PositionError::trigger_close(
                Percent100::from_ratio(super::lease_coin(919), super::lease_coin(1000 * 2)),
                CloseStrategy::TakeProfit(take_profit_trigger)
            )),
            spec.change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(take_profit_trigger)),
                    stop_loss: None,
                },
                super::lease_coin(1000),
                &super::due(919, 0),
                super::price_from_amount(1, 2),
            )
        );
        let spec = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Set(take_profit_trigger)),
                    stop_loss: None,
                },
                super::lease_coin(1000),
                &super::due(920, 0),
                super::price_from_amount(1, 2),
            )
            .unwrap();

        assert_eq!(
            None,
            // 92 LPNs due = 46 Asset units due, 46/100 = 46% LPN
            spec.check_close(asset, &super::due(92, 0), super::price_from_amount(1, 2))
        );

        assert_eq!(
            Some(CloseStrategy::TakeProfit(take_profit_trigger)),
            // 90 LPNs due = 45 Asset units due, 45/100 = 45% LPN
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );

        let spec = spec
            .change_close_policy(
                ClosePolicyChange {
                    take_profit: Some(ChangeCmd::Reset),
                    stop_loss: Some(ChangeCmd::Set(take_profit_trigger)),
                },
                super::lease_coin(1000),
                &super::due(550, 0),
                super::price_from_amount(1, 2),
            )
            .unwrap();

        assert_eq!(
            None,
            // 90 LPNs due = 45 Asset units due, 45/100 = 45% LPN
            spec.check_close(asset, &super::due(90, 0), super::price_from_amount(1, 2))
        );
        assert_eq!(
            Some(CloseStrategy::StopLoss(take_profit_trigger)),
            // 92 LPNs due = 46 Asset units due, 46/100 = 46% LPN
            spec.check_close(asset, &super::due(92, 0), super::price_from_amount(1, 2))
        );
    }
}
