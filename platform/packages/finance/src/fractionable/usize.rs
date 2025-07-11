use crate::{
    percent::{Units as PercentUnits, bound::BoundPercent},
    ratio::RatioLegacy,
    traits::FractionUnit,
};

use super::Fractionable;

impl<const UPPER_BOUND: PercentUnits> Fractionable<BoundPercent<UPPER_BOUND>> for usize {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<BoundPercent<UPPER_BOUND>>,
    {
        u128::try_from(self)
            .expect("usize to u128 overflow")
            .safe_mul(fraction)
            .try_into()
            .expect("usize overflow on percent calculation")
    }
}

impl FractionUnit for usize {}

#[cfg(test)]
mod test {
    use crate::{
        fraction::Fraction,
        percent::{Percent, Percent100},
        rational::Rational,
    };

    #[test]
    fn ok() {
        let n = 123usize;
        assert_eq!(n, Percent100::HUNDRED.of(n));
        assert_eq!(n / 2, Percent100::from_percent(50).of(n));
        assert_eq!(n * 3 / 2, Percent100::from_percent(150).of(n));

        assert_eq!(usize::MAX, Percent100::HUNDRED.of(usize::MAX));
        assert_eq!(usize::MIN, Percent100::from_permille(1).of(999));
        assert_eq!(usize::MAX / 2, Percent100::from_percent(50).of(usize::MAX));
    }

    #[test]
    #[should_panic = "usize overflow"]
    fn overflow() {
        _ = Percent::from_permille(1001).of(usize::MAX);
    }
}
