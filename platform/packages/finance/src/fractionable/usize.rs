use crate::{percent::Units as PercentUnits, ratio::Ratio};

use super::Fractionable;

impl Fractionable<PercentUnits> for usize {
    fn safe_mul(self, fraction: &Ratio<PercentUnits>) -> Self {
        u128::try_from(self)
            .expect("usize to u128 overflow")
            .safe_mul(fraction)
            .try_into()
            .expect("usize overflow on percent calculation")
    }
}

#[cfg(test)]
mod test {
    use crate::{fraction::Fraction, percent::Percent100};

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
}
