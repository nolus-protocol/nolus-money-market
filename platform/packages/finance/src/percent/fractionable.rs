use crate::{
    coin::{Coin, DoubleCoinPrimitive},
    fractionable::{CommonDoublePrimitive, Fractionable, IntoDoublePrimitive, IntoMax, TryFromMax},
    percent::{Units, bound::BoundPercent, permilles::Permilles},
};

pub(crate) type DoubleBoundPercentPrimitive = u64;

impl<C> CommonDoublePrimitive<Coin<C>> for Permilles {
    type CommonDouble = DoubleCoinPrimitive;
}

impl CommonDoublePrimitive<Permilles> for Permilles {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<const UPPER_BOUND: Units> CommonDoublePrimitive<Permilles> for BoundPercent<UPPER_BOUND> {
    type CommonDouble = DoubleBoundPercentPrimitive;
}

impl<C> Fractionable<Coin<C>> for Permilles {}

impl Fractionable<Permilles> for Permilles {}
impl<const UPPER_BOUND: Units> Fractionable<Permilles> for BoundPercent<UPPER_BOUND> {}

impl<const UPPER_BOUND: Units> IntoDoublePrimitive for BoundPercent<UPPER_BOUND> {
    type Double = DoubleBoundPercentPrimitive;

    fn into_double(self) -> Self::Double {
        Permilles::from(self).into_double()
    }
}

impl IntoDoublePrimitive for Permilles {
    type Double = DoubleBoundPercentPrimitive;

    fn into_double(self) -> Self::Double {
        self.units().into()
    }
}

impl<const UPPER_BOUND: Units> IntoMax<DoubleBoundPercentPrimitive> for BoundPercent<UPPER_BOUND> {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.into_double()
    }
}

impl IntoMax<DoubleBoundPercentPrimitive> for Permilles {
    fn into_max(self) -> DoubleBoundPercentPrimitive {
        self.into_double()
    }
}

impl IntoMax<DoubleCoinPrimitive> for Permilles {
    fn into_max(self) -> DoubleCoinPrimitive {
        self.into_double().into()
    }
}

impl<const UPPER_BOUND: Units> TryFromMax<DoubleBoundPercentPrimitive>
    for BoundPercent<UPPER_BOUND>
{
    fn try_from_max(max: DoubleBoundPercentPrimitive) -> Option<Self> {
        Units::try_from(max)
            .ok()
            .map(Permilles::new)
            .and_then(|permilles| Self::try_from(permilles).ok())
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
            percent::{DoubleBoundPercentPrimitive, HUNDRED, Percent, Percent100, Units},
            rational::Rational,
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent100::from_permille(410 * 222 / HUNDRED),
                Percent100::from_percent(41).of(Percent100::from_permille(222))
            );
            assert_eq!(
                Percent100::from_permille(999),
                Percent100::from_percent(100).of(Percent100::from_permille(999))
            );
            assert_eq!(
                Percent::from_permille(410 * 222222 / HUNDRED),
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
            percent::{Units, permilles::Permilles},
            ratio::{Ratio, SimpleFraction},
            rational::Rational,
            test::coin,
        };

        #[test]
        fn of() {
            assert_eq!(
                Permilles::new(Units::MAX),
                SimpleFraction::new(coin::coin1(Amount::MAX), coin::coin1(Amount::MAX))
                    .of(Permilles::new(Units::MAX))
                    .unwrap()
            );
            assert_eq!(
                Permilles::new(1500),
                Ratio::new(coin::coin1(3), coin::coin1(4)).of(Permilles::new(2000))
            );
            assert_eq!(
                Permilles::new(200),
                Ratio::new(coin::coin1(1), coin::coin1(5)).of(Permilles::MILLE)
            );
            assert_eq!(
                Permilles::new(225),
                SimpleFraction::new(coin::coin1(3), coin::coin1(2))
                    .of(Permilles::new(150))
                    .unwrap()
            );
        }

        #[test]
        fn of_overflow() {
            assert!(
                SimpleFraction::new(coin::coin1(Amount::MAX), coin::coin1(1))
                    .of(Permilles::new(10))
                    .is_none()
            )
        }
    }
}
