use crate::{
    coin::{Amount, Coin},
    percent::{Percent, Units},
};

use super::Fractionable;

impl Fractionable<Percent> for Percent {
    type MaxRank = Units;
}

impl<C> Fractionable<Coin<C>> for Percent {
    type MaxRank = Amount;
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
                Percent::from_percent(41).safe_mul(&Percent::from_permille(222222))
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
            assert_eq!(
                Percent::from_percent(u16::MAX),
                Percent::from_percent(100).safe_mul(&Percent::from_percent(u16::MAX))
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
                Fractionable::<Coin<_>>::safe_mul(Percent::from_permille(Units::MAX), &ratio_one)
            );
        }
    }
}
