use crate::{
    coin::Coin,
    fraction::Fraction,
    percent::{Units, bound::BoundPercent},
    ratio::Ratio,
    rational::Rational,
};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u32
where
    T: Into<Self>,
{
    type Type = u64;
    type Intermediate = Self;
}

impl<const UPPER_BOUND: Units> Fractionable<BoundPercent<UPPER_BOUND>>
    for BoundPercent<UPPER_BOUND>
{
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<BoundPercent<UPPER_BOUND>>,
    {
        self.of(ratio)
    }
}

impl<C, const UPPER_BOUND: Units> Fractionable<Coin<C>> for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Ratio<Coin<C>>,
    {
        let p128: u128 = self.units().into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        let res: Units = p128
            .safe_mul(fraction)
            .try_into()
            .expect("overflow computing a fraction of permille");
        Self::from_permille(res)
    }
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fraction::Fraction,
            fractionable::HigherRank,
            percent::{Percent100, Units},
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent100::from_permille(410 * 222222 / 1000),
                Percent100::from_percent(41).of(Percent100::from_permille(222222))
            );

            let p_units: Units = 410;
            let p64: <u32 as HigherRank<u8>>::Type = p_units.into();
            let p64_res: <u32 as HigherRank<u8>>::Type = p64 * u64::from(Units::MAX) / 1000;
            let p_units_res: Units = p64_res.try_into().expect("u64 -> Units overflow");
            assert_eq!(
                Percent100::from_permille(p_units_res),
                Percent100::from_percent(41).of(Percent100::from_permille(Units::MAX))
            );
        }

        #[test]
        fn of_hundred_percent() {
            assert_eq!(
                Percent100::from_permille(999),
                Percent100::from_percent(100).of(Percent100::from_permille(999))
            );
        }

        #[test]
        #[should_panic]
        fn of_overflow() {
            Percent100::from_permille(1001).of(Percent100::from_permille(Units::MAX));
        }
    }

    mod rational {
        use currency::test::SuperGroupTestC1;

        use crate::{coin::Coin, percent::Percent100, ratio::SimpleFraction, rational::Rational};

        #[test]
        fn of() {
            // TODO replace it with Ratio whe it becode a struct
            let ratio_one = SimpleFraction::new(
                Coin::<SuperGroupTestC1>::new(u128::MAX),
                Coin::<SuperGroupTestC1>::new(u128::MAX),
            );
            assert_eq!(
                Percent100::from_permille(899),
                ratio_one.of(Percent100::from_permille(899)).unwrap()
            );
        }
    }
}
