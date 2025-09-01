use crate::{fraction::Unit as FractionUnit, percent::Units as PercentUnits, ratio::RatioLegacy};

use super::Fractionable;

impl Fractionable<PercentUnits> for usize {
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: RatioLegacy<PercentUnits>,
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
    };

    #[test]
    fn ok() {
        let n = 123usize;
        assert_eq!(n, Percent100::HUNDRED.of(n));
        assert_eq!(n / 2, Percent100::from_percent(50).of(n));
        assert_eq!(n * 3 / 4, Percent100::from_percent(75).of(n));

        assert_eq!(usize::MAX, Percent100::HUNDRED.of(usize::MAX));
        assert_eq!(usize::MIN, Percent100::from_permille(1).of(999));
        assert_eq!(usize::MAX / 2, Percent100::from_percent(50).of(usize::MAX));
    }

    #[test]
    #[should_panic = "usize overflow"]
    fn overflow() {
        use crate::rational::Rational;

        // TODO remove the `#[should_panic]` and assert that is None when
        // SimpleFraction::of() calls its checked_mul method instead of safe_mul
        _ = Percent::from_permille(1001).of(usize::MAX);
    }
}
