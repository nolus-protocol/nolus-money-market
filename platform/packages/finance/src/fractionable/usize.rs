#[cfg(test)]
use crate::arithmetics::CheckedMul;
use crate::{
    fraction::Unit as FractionUnit,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::Units as PercentUnits,
};

// TODO: Remove when protocol/marketplace gets refactored to use a type based on u32
#[cfg(not(test))]
impl Fractionable<PercentUnits> for usize {
    type HigherPrimitive = u64;
}

#[cfg(not(test))]
impl ToPrimitive<u64> for usize {
    fn into_primitive(self) -> u64 {
        self.try_into().expect("usize is greater than u64")
    }
}

#[cfg(not(test))]
impl TryFromPrimitive<u64> for usize {
    fn try_from_primitive(primitive: u64) -> Option<Self> {
        primitive.try_into().ok()
    }
}

// Test purposes
// remove with  `usize` removal
#[cfg(test)]
impl Fractionable<PercentUnits> for usize {
    type HigherPrimitive = u128;
}
#[cfg(test)]
impl ToPrimitive<u128> for u32 {
    fn into_primitive(self) -> u128 {
        self.into()
    }
}

#[cfg(test)]
impl ToPrimitive<u128> for usize {
    fn into_primitive(self) -> u128 {
        self.try_into().expect("usize is greater than u64")
    }
}

#[cfg(test)]
impl TryFromPrimitive<u128> for usize {
    fn try_from_primitive(primitive: u128) -> Option<Self> {
        primitive.try_into().ok()
    }
}

#[cfg(test)]
impl CheckedMul for u128 {
    type Output = u128;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}

impl FractionUnit for usize {}

#[cfg(test)]
mod test {
    use std::usize;

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
    fn overflow() {
        use crate::rational::Rational;

        assert!(Rational::of(&Percent::from_permille(1001), usize::MAX).is_none());
    }
}
