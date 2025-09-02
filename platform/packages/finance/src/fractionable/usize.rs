use gcd::Gcd;

use crate::{
    percent::Units as PercentUnits,
    ratio::{RatioLegacy, Scalar},
};

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

impl Scalar for usize {
    type Times = Self;

    fn gcd(self, other: Self) -> Self::Times {
        Gcd::gcd(self, other)
    }

    fn scale_up(self, scale: Self::Times) -> Option<Self> {
        self.checked_mul(scale)
    }

    fn scale_down(self, scale: Self::Times) -> Self {
        debug_assert_ne!(scale, 0);

        self / scale
    }

    fn modulo(self, scale: Self::Times) -> Self::Times {
        debug_assert_ne!(scale, 0);

        self % scale
    }

    fn into_times(self) -> Self::Times {
        self
    }
}

#[cfg(test)]
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
