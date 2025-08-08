use bnum::types::U256;

use crate::{
    arithmetics::CheckedMul,
    coin::Coin,
    fractionable::{Fractionable, ToPrimitive, TryFromPrimitive},
    percent::{Units, bound::BoundPercent},
    ratio::{Ratio, SimpleFraction},
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
        u128::from(self.units()).into()
    }
}

impl<const UPPER_BOUND: Units> ToPrimitive<SimpleFraction<U256>> for BoundPercent<UPPER_BOUND> {
    fn into_primitive(self) -> SimpleFraction<U256> {
        self._to_fraction::<U256>()
    }
}

impl<const UPPER_BOUND: Units> TryFromPrimitive<u64> for BoundPercent<UPPER_BOUND> {
    fn try_from_primitive(primitive: u64) -> Option<Self> {
        Units::try_from(primitive).ok().map(Self::from_permille)
    }
}

impl<const UPPER_BOUND: Units> TryFromPrimitive<U256> for BoundPercent<UPPER_BOUND> {
    fn try_from_primitive(primitive: U256) -> Option<Self> {
        u128::try_from(primitive)
            .ok()
            .and_then(|u_128| Units::try_from(u_128).ok().map(Self::from_permille))
    }
}

// TODO implement Fractionble<BoundPercent<UPPER_BOUND>> for BoundPercent<UPPER_BOUND>
impl<const UPPER_BOUND: Units> Fragmentable<Units> for BoundPercent<UPPER_BOUND> {
    #[track_caller]
    fn safe_mul<R>(self, ratio: &R) -> Self
    where
        R: Ratio<Units>,
    {
        Self::try_from(self.units().safe_mul(ratio))
            .expect("Resulting permille exceeds BoundPercent upper bound")
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
        Self::try_from(res).expect("Resulting permille exceeds BoundPercent upper bound")
    }
}
