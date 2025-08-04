use bnum::types::U256;

use crate::{
    arithmetics::CheckedMul,
    coin::Coin,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::{Units, bound::BoundPercent},
    ratio::Ratio,
};

use super::{Fragmentable, HigherRank};

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
    type HigherPrimitive = u64;
}

impl<C, const UPPER_BOUND: Units> Fractionable<Coin<C>> for BoundPercent<UPPER_BOUND> {
    type HigherPrimitive = U256;
}

impl CheckedMul<u64> for u64 {
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}

impl CheckedMul<U256> for U256 {
    type Output = U256;

    fn checked_mul(self, rhs: U256) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}

impl<const UPPER_BOUND: Units> ToPrimitive<u64> for BoundPercent<UPPER_BOUND> {
    fn into_primitive(self) -> u64 {
        self.units().into()
    }
}

impl<const UPPER_BOUND: Units> ToPrimitive<U256> for BoundPercent<UPPER_BOUND> {
    fn into_primitive(self) -> U256 {
        u128::from(self).into()
    }
}

impl<const UPPER_BOUND: Units> TryFromPrimitive<u64> for BoundPercent<UPPER_BOUND> {
    fn try_from_primitive(primitive: u64) -> Option<Self> {
        Units::try_from(primitive)
            .ok()
            .map(|units| Self::from_permille(units))
    }
}

impl<const UPPER_BOUND: Units> TryFromPrimitive<U256> for BoundPercent<UPPER_BOUND> {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        u128::try_from(primitive).ok().and_then(|u_128| {
            Units::try_from(u_128)
                .ok()
                .map(|units| Self::from_permille(units))
        })
    }
}

// TODO implement Fractionble<BoundPercent<UPPER_BOUND>> for BoundPercent<UPPER_BOUND>
impl<const UPPER_BOUND: Units> Fragmentable<Units> for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<Units>,
    {
        Self::from_permille(self.units().safe_mul(ratio))
    }
}

impl<C, const UPPER_BOUND: Units> Fragmentable<Coin<C>> for BoundPercent<UPPER_BOUND> {
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
            percent::{Percent, Percent100, Units},
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
            use crate::rational::Rational;

            Percent::from_permille(1001).of(Percent::from_permille(Units::MAX));
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
