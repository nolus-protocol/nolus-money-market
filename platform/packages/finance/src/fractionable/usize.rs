use crate::percent::{Percent, Units as PercentUnits};

use super::Fractionable;

impl Fractionable<Percent> for usize {
    type MaxRank = PercentUnits;
}

/* #[cfg(test)]
mod test {
    use crate::{fraction::Fraction, percent::Percent};

    #[test]
    fn ok() {
        let n = 123usize;
        assert_eq!(n, Percent::HUNDRED.of(n));
        assert_eq!(n / 2, Percent::from_percent(50).of(n));
        assert_eq!(n * 3 / 2, Percent::from_percent(150).of(n));

        assert_eq!(usize::MAX, Percent::HUNDRED.of(usize::MAX));
        assert_eq!(usize::MIN, Percent::from_permille(1).of(999));
        assert_eq!(usize::MAX / 2, Percent::from_percent(50).of(usize::MAX));
    }

    #[test]
    #[should_panic = "usize overflow"]
    fn overflow() {
        _ = Percent::from_permille(1001).of(usize::MAX);
    }
}
 */