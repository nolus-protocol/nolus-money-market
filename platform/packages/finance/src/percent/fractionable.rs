use crate::{
    coin::{Coin, DoubleCoinPrimitive},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoDoublePrimitive, IntoMax, TryFromMax},
    percent::{Units, bound::BoundPercent, permilles::Permilles},
};

pub(crate) type DoubleBoundPercentPrimitive = u64;

impl<C> CommonDoublePrimitive<Coin<C>> for Permilles {
    type CommonDouble = DoubleCoinPrimitive;
}

impl<const UPPER_BOUND: Units> CommonDoublePrimitive<Permilles> for BoundPercent<UPPER_BOUND> {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<C> Fractionable<Coin<C>> for Permilles {}

impl IntoMax<DoubleBoundPercentPrimitive> for Permilles {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.into_double().into()
    }
}

impl IntoMax<DoubleCoinPrimitive> for Permilles {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.into_double().into()
    }
}

impl IntoDoublePrimitive for Permilles {
    type Double = DoubleBoundPercentPrimitive;

    fn into_double(self) -> Self::Double {
        self.units().into()
    }
}

impl TryFromMax<DoubleBoundPercentPrimitive> for Permilles {
    fn try_from_max(max: DoubleBoundPercentPrimitive) -> Option<Self> {
        Units::try_from(max).ok().map(Self::new)
    }
}

impl TryFromMax<DoubleCoinPrimitive> for Permilles {
    fn try_from_max(max: DoubleCoinPrimitive) -> Option<Self> {
        Units::try_from(max).ok().map(Self::new)
    }
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fraction::Fraction,
            fractionable::{IntoDoublePrimitive, TryFromMax},
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
            let p64 = percent.into_double();
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
