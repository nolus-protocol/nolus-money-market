use bnum::types::U256;

use crate::{
    coin::Coin,
    fractionable::{MaxDoublePrimitive, ToDoublePrimitive},
    percent::{Units, bound::BoundPercent},
    ratio::RatioLegacy,
};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u32
where
    T: Into<Self>,
{
    type Type = u64;
}

impl<const UPPER_BOUND: Units> Fractionable<Units> for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: RatioLegacy<Units>,
    {
        Self::try_from(self.units().safe_mul(ratio))
            .expect("TODO remove when refactor Fractionable. Resulting permille exceeds BoundPercent upper bound")
    }
}

impl<const UPPER_BOUND: Units> ToDoublePrimitive for BoundPercent<UPPER_BOUND> {
    type Double = u64;

    fn to_double(self) -> Self::Double {
        self.units().into()
    }
}

impl<C, const UPPER_BOUND: Units> MaxDoublePrimitive<Coin<C>> for BoundPercent<UPPER_BOUND> {
    type Max = U256;

    fn into_max_self(self) -> Self::Max {
        self.to_double().into()
    }

    fn into_max_other(other: Coin<C>) -> Self::Max {
        other.to_double()
    }

    fn try_from_max(max: Self::Max) -> Option<Self> {
        u128::try_from(max).ok().and_then(|u_128| {
            Units::try_from(u_128)
                .ok()
                .and_then(Self::try_from_permille)
        })
    }
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fractionable::{Fractionable, HigherRank},
            percent::{Percent, Percent100, Units},
        };

        #[test]
        fn safe_mul() {
            assert_eq!(
                Percent100::from_permille(410 * 222 / 1000),
                Percent100::from_percent(41).safe_mul(&Percent100::from_permille(222))
            );
            assert_eq!(
                Percent100::from_permille(999),
                Percent100::from_percent(100).safe_mul(&Percent100::from_permille(999))
            );
            assert_eq!(
                Percent::from_permille(410 * 222222 / 1000),
                Percent::from_percent(41).safe_mul(&Percent::from_permille(222222))
            );
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Percent::from_percent(100).safe_mul(&Percent::from_permille(Units::MAX))
            );

            let p_units: Units = 410;
            let p64: <u32 as HigherRank<u8>>::Type = p_units.into();
            let p64_res: <u32 as HigherRank<u8>>::Type = p64 * u64::from(Units::MAX) / 1000;
            let p_units_res: Units = p64_res.try_into().expect("u64 -> Units overflow");

            assert_eq!(
                Percent::from_permille(p_units_res),
                Percent::from_percent(41).safe_mul(&Percent::from_permille(Units::MAX))
            );
        }

        #[test]
        fn safe_mul_hundred_percent() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Percent::from_percent(100).safe_mul(&Percent::from_permille(Units::MAX))
            );
        }

        #[test]
        #[should_panic]
        fn safe_mul_overflow() {
            Percent::from_permille(1001).safe_mul(&Percent::from_permille(Units::MAX));
        }
    }

    mod rational {
        use currency::test::SuperGroupTestC1;

        use crate::{
            coin::Coin,
            fraction::Fraction,
            percent::{Percent, Percent100, Units},
            ratio::{Ratio, SimpleFraction},
            rational::Rational,
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                SimpleFraction::new(
                    Coin::<SuperGroupTestC1>::new(u128::MAX),
                    Coin::<SuperGroupTestC1>::new(u128::MAX)
                )
                .of(Percent::from_permille(Units::MAX))
                .unwrap()
            );
            assert_eq!(
                Percent100::from_percent(20),
                Ratio::new(
                    Coin::<SuperGroupTestC1>::new(1),
                    Coin::<SuperGroupTestC1>::new(5),
                )
                .of(Percent100::HUNDRED)
            );
            assert_eq!(
                Percent::from_permille(225),
                SimpleFraction::new(
                    Coin::<SuperGroupTestC1>::new(3),
                    Coin::<SuperGroupTestC1>::new(2),
                )
                .of(Percent::from_permille(150))
                .unwrap()
            );
        }

        #[test]
        fn of_overflow() {
            assert!(
                SimpleFraction::new(
                    Coin::<SuperGroupTestC1>::new(u128::MAX),
                    Coin::<SuperGroupTestC1>::new(1),
                )
                .of(Percent::from_percent(1))
                .is_none()
            )
        }
    }
}
