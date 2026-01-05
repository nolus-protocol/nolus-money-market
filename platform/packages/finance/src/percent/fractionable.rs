use crate::{
    coin::{Coin, DoubleCoinPrimitive},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoMax, ToDoublePrimitive, TryFromMax},
    percent::{Units, bound::BoundPercent},
};

pub(crate) type DoubleBoundPercentPrimitive = u64;

impl<const UPPER_BOUND: Units> CommonDoublePrimitive<Self> for BoundPercent<UPPER_BOUND> {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<C, const UPPER_BOUND: Units> CommonDoublePrimitive<Coin<C>> for BoundPercent<UPPER_BOUND> {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<const UPPER_BOUND: Units> Fractionable<Self> for BoundPercent<UPPER_BOUND> {}

impl<C, const UPPER_BOUND: Units> Fractionable<Coin<C>> for BoundPercent<UPPER_BOUND> {}

impl<const UPPER_BOUND: Units> IntoMax<DoubleBoundPercentPrimitive> for BoundPercent<UPPER_BOUND> {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.to_double()
    }
}

impl<const UPPER_BOUND: Units> IntoMax<DoubleCoinPrimitive> for BoundPercent<UPPER_BOUND> {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.to_double().into()
    }
}

impl<const UPPER_BOUND: Units> ToDoublePrimitive for BoundPercent<UPPER_BOUND> {
    type Double = DoubleBoundPercentPrimitive;

    fn to_double(self) -> Self::Double {
        self.units().into()
    }
}

impl<const UPPER_BOUND: Units> TryFromMax<DoubleBoundPercentPrimitive>
    for BoundPercent<UPPER_BOUND>
{
    fn try_from_max(max: DoubleBoundPercentPrimitive) -> Option<Self> {
        Units::try_from(max)
            .ok()
            .and_then(|units| Self::try_from(units).ok())
    }
}

impl<const UPPER_BOUND: Units> TryFromMax<DoubleCoinPrimitive> for BoundPercent<UPPER_BOUND> {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        Units::try_from(max)
            .ok()
            .and_then(|units| Self::try_from(units).ok())
    }
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fraction::Fraction,
            fractionable::{ToDoublePrimitive, TryFromMax},
            percent::{DoubleBoundPercentPrimitive, Percent, Percent100, Units},
            rational::Rational,
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent100::from_permille(410 * 222 / 1000),
                Percent100::from_percent(41).of(Percent100::from_permille(222))
            );
            assert_eq!(
                Percent100::from_permille(999),
                Percent100::from_percent(100).of(Percent100::from_permille(999))
            );
            assert_eq!(
                Percent::from_permille(410 * 222222 / 1000),
                Percent::from_percent(41)
                    .of(Percent::from_permille(222222))
                    .unwrap()
            );
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Percent::from_percent(100)
                    .of(Percent::from_permille(Units::MAX))
                    .unwrap()
            );

            let percent = Percent::from_permille(410);
            let p64 = percent.to_double();
            let p64_res = p64 * DoubleBoundPercentPrimitive::from(Units::MAX) / 1000;
            let percent_res = Percent::try_from_max(p64_res)
                .expect("DoubleBoundPercentPrimitive -> Percent overflow");

            assert_eq!(
                percent_res,
                Percent::from_percent(41)
                    .of(Percent::from_permille(Units::MAX))
                    .unwrap()
            );
        }

        #[test]
        fn of_hundred_percent() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Percent::from_percent(100)
                    .of(Percent::from_permille(Units::MAX))
                    .unwrap()
            );
        }

        #[test]
        fn of_overflow() {
            assert!(
                Percent::from_permille(1001)
                    .of(Percent::from_permille(Units::MAX))
                    .is_none()
            )
        }
    }

    mod fraction {

        use crate::{
            coin::Amount,
            fraction::Fraction,
            percent::{Percent, Percent100, Units},
            ratio::{Ratio, SimpleFraction},
            rational::Rational,
            test::coin,
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                SimpleFraction::new(coin::coin1(Amount::MAX), coin::coin1(Amount::MAX))
                    .of(Percent::from_permille(Units::MAX))
                    .unwrap()
            );
            assert_eq!(
                Percent::from_permille(1500),
                Ratio::new(coin::coin1(3), coin::coin1(4)).of(Percent::from_permille(2000))
            );
            assert_eq!(
                Percent100::from_percent(20),
                Ratio::new(coin::coin1(1), coin::coin1(5)).of(Percent100::HUNDRED)
            );
            assert_eq!(
                Percent100::from_permille(225),
                SimpleFraction::new(coin::coin1(3), coin::coin1(2))
                    .of(Percent100::from_permille(150))
                    .unwrap()
            );
        }

        #[test]
        fn of_overflow() {
            assert!(
                SimpleFraction::new(coin::coin1(Amount::MAX), coin::coin1(1))
                    .of(Percent::from_percent(1))
                    .is_none()
            )
        }
    }
}
