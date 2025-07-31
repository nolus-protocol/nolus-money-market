/// Abstraction over checked multiplication for primitive types.
/// Used to safely multiply values with overflow detection.
pub trait CheckedMul<Rhs = Self> {
    type Output;

    fn checked_mul(self, rhs: Rhs) -> Option<Self::Output>;
}
