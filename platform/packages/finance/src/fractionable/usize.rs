use crate::{percent::Units as PercentUnits, ratio::Ratio};

use super::{Fractionable, HigherRank};

impl<T> HigherRank<T> for u64
where
    T: Into<Self>,
{
    type Type = u128;
    type Intermediate = Self;
}

impl Fractionable<PercentUnits> for usize {
    fn checked_mul<F>(self, fraction: &F) -> Option<Self>
    where
        F: Ratio<PercentUnits>,
    {
        u128::try_from(self).ok().and_then(|units| {
            Fractionable::<PercentUnits>::checked_mul(units, fraction)
                .and_then(|units| units.try_into().ok())
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{fraction::Fraction, percent::Percent};

    #[test]
    fn ok() {
        let n = 123usize;
        assert_eq!(n, Percent::HUNDRED.of(n).unwrap());
        assert_eq!(n / 2, Percent::from_percent(50).of(n).unwrap());
        assert_eq!(n * 3 / 2, Percent::from_percent(150).of(n).unwrap());

        assert_eq!(usize::MAX, Percent::HUNDRED.of(usize::MAX).unwrap());
        assert_eq!(usize::MIN, Percent::from_permille(1).of(999).unwrap());
        assert_eq!(
            usize::MAX / 2,
            Percent::from_percent(50).of(usize::MAX).unwrap()
        );
    }

    #[test]
    fn overflow() {
        assert!(Percent::from_permille(1001).of(usize::MAX).is_err())
    }
}
