use crate::{
    coin::Coin,
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

impl<C, const UPPER_BOUND: Units> Fractionable<Coin<C>> for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<Coin<C>>,
    {
        let p128: u128 = self.units().into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        let res: Units = p128
            .safe_mul(fraction)
            .try_into()
            .expect("overflow computing a fraction of permille");
        Self::try_from(res).expect("TODO remove when refactor Fractionable. Resulting permille exceeds BoundPercent upper bound")
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
            fractionable::Fractionable,
            percent::{Percent, Units},
            ratio::SimpleFraction,
        };

        #[test]
        fn safe_mul() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Fractionable::<Coin<_>>::safe_mul(
                    Percent::from_permille(Units::MAX),
                    &SimpleFraction::new(
                        Coin::<SuperGroupTestC1>::new(u128::MAX),
                        Coin::<SuperGroupTestC1>::new(u128::MAX),
                    )
                )
            );
            assert_eq!(
                Percent::from_percent(20),
                Fractionable::<Coin<_>>::safe_mul(
                    Percent::HUNDRED,
                    &SimpleFraction::new(
                        Coin::<SuperGroupTestC1>::new(1),
                        Coin::<SuperGroupTestC1>::new(5),
                    )
                )
            );
            assert_eq!(
                Percent::from_permille(225),
                Fractionable::<Coin<_>>::safe_mul(
                    Percent::from_permille(150),
                    &SimpleFraction::new(
                        Coin::<SuperGroupTestC1>::new(3),
                        Coin::<SuperGroupTestC1>::new(2),
                    )
                )
            );
        }

        #[test]
        #[should_panic]
        fn safe_mul_overflow() {
            Fractionable::<Coin<_>>::safe_mul(
                Percent::from_percent(1),
                &SimpleFraction::new(
                    Coin::<SuperGroupTestC1>::new(u128::MAX),
                    Coin::<SuperGroupTestC1>::new(1),
                ),
            );
        }
    }
}
