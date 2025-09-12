pub trait CheckedMul<Rhs = Self> {
    type Output;

    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}
