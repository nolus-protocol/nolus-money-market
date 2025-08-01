use crate::{
    coin::Coin,
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
