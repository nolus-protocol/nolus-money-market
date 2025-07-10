use crate::{
    coin::{Amount, Coin},
    percent::{Units, bound::BoundPercent},
};

use super::Fractionable;

impl<const UPPER_BOUND: Units> Fractionable<BoundPercent<UPPER_BOUND>>
    for BoundPercent<UPPER_BOUND>
{
    type MaxRank = Units;
}

impl<C, const UPPER_BOUND: Units> Fractionable<Coin<C>> for BoundPercent<UPPER_BOUND> {
    type MaxRank = Amount;
}

#[cfg(test)]
mod test {
    mod percent {
        use crate::{
            fraction::Fraction,
            percent::{Percent100, Units},
        };

        #[test]
        fn of() {
            assert_eq!(
                Percent100::from_permille(410 * 222 / 1000),
                Percent100::from_percent(41).of(Percent100::from_permille(222))
            );

            let p_units: Units = 410;
            let p64: u64 = p_units.into();
            let p64_res = p64 * u64::from(100 as Units) / 1000;
            let p_units_res: Units = p64_res.try_into().expect("u64 -> Units overflow");

            assert_eq!(
                Percent100::from_permille(p_units_res),
                Percent100::from_permille(50).of(Percent100::from_percent(82))
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

        use crate::{coin::Coin, fraction::Fraction, percent::Percent100, ratio::Ratio};

        #[test]
        fn safe_mul() {
            let ratio_one = Ratio::new(
                Coin::<SuperGroupTestC1>::new(u128::MAX),
                Coin::<SuperGroupTestC1>::new(u128::MAX),
            );
            assert_eq!(
                Percent100::from_permille(899),
                ratio_one.of(Percent100::from_permille(899))
            );
        }
    }
}
