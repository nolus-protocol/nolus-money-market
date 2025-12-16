use crate::coin::DoubleCoinPrimitive;

pub trait CheckedMul<Rhs = Self> {
    type Output;

    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}

impl CheckedMul for u64 {
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}

impl CheckedMul for DoubleCoinPrimitive {
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}
