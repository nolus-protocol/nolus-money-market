use crate::{
    coin::Coin,
    percent::{Percent, Units},
    ratio::Ratio,
};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u32
where
    T: Into<Self>,
{
    type Type = u64;
    type Intermediate = Self;
}

impl Fractionable<Units> for Percent {
    #[track_caller]
    fn checked_mul<R>(self, ratio: &R) -> Option<Self>
    where
        R: Ratio<Units>,
    {
        Fractionable::<Units>::checked_mul(self.units(), ratio).map(Self::from_permille)
    }
}

impl<C> Fractionable<Coin<C>> for Percent {
    #[track_caller]
    fn checked_mul<F>(self, fraction: &F) -> Option<Self>
    where
        F: Ratio<Coin<C>>,
    {
        let p128: u128 = self.units().into();
        // TODO re-assess the design of Ratio ... and whether it could be > 1
        Fractionable::<Coin<C>>::checked_mul(p128, fraction)
            .and_then(|units| units.try_into().ok().map(Self::from_permille))
    }
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fractionable::{Fractionable, HigherRank},
            percent::{Percent, Units},
        };

        #[test]
        fn safe_mul() {
            assert_eq!(
                Percent::from_permille(410 * 222222 / 1000),
                Percent::from_percent(41)
                    .checked_mul(&Percent::from_permille(222222))
                    .unwrap()
            );

            let p_units: Units = 410;
            let p64: <u32 as HigherRank<u8>>::Type = p_units.into();
            let p64_res: <u32 as HigherRank<u8>>::Type = p64 * u64::from(Units::MAX) / 1000;
            let p_units_res: Units = p64_res.try_into().expect("u64 -> Units overflow");
            assert_eq!(
                Percent::from_permille(p_units_res),
                Percent::from_percent(41)
                    .checked_mul(&Percent::from_permille(Units::MAX))
                    .unwrap()
            );
        }

        #[test]
        fn safe_mul_hundred_percent() {
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Percent::from_percent(100)
                    .checked_mul(&Percent::from_permille(Units::MAX))
                    .unwrap()
            );
            assert_eq!(
                Percent::from_percent(u16::MAX),
                Percent::from_percent(100)
                    .checked_mul(&Percent::from_percent(u16::MAX))
                    .unwrap()
            );
        }

        #[test]
        fn safe_mul_overflow() {
            assert!(
                Percent::from_permille(1001)
                    .checked_mul(&Percent::from_permille(Units::MAX))
                    .is_none(),
                "Multiplication did not overflow as expected"
            )
        }
    }

    mod rational {
        use currency::test::SuperGroupTestC1;

        use crate::{
            coin::Coin,
            fractionable::Fractionable,
            percent::{Percent, Units},
            ratio::Rational,
        };

        #[test]
        fn safe_mul() {
            let ratio_one = Rational::new(
                Coin::<SuperGroupTestC1>::new(u128::MAX),
                Coin::<SuperGroupTestC1>::new(u128::MAX),
            );
            assert_eq!(
                Percent::from_permille(Units::MAX),
                Fractionable::<Coin<_>>::checked_mul(
                    Percent::from_permille(Units::MAX),
                    &ratio_one
                )
                .unwrap()
            );
        }
    }
}
