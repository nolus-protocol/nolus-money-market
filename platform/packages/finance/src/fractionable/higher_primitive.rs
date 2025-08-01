use bnum::types::U256;

use crate::arithmetics::CheckedMul;

impl CheckedMul for U256 {
    type Output = Self;

    fn checked_mul(self, rhs: Self) -> Option<Self::Output> {
        self.checked_mul(rhs)
    }
}
