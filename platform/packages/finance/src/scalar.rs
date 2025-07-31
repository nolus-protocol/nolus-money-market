use std::fmt::Debug;

// TODO: Write a blanket implementation if separate implementations blocks look similar
/// Enables arithmetic operations on business types through their underlying primitive representation
pub trait Scalable
where
    Self: Copy,
{
    /// The primitive representation of `Self`
    type Times: Copy + Debug;

    /// Multiplies `self` by the provided `scalar`.
    ///
    /// Returns `None` if an overflow occurs.
    fn scale_up(self, scalar: Self::Times) -> Option<Self>;

    // TODO: Change return type `Option<Self> --> Self` if no undefined behavior `.div(0)` is expected
    /// Divides `self` by the provided `scalar`.
    ///
    /// Returns `None` if division by zero occurs.
    fn scale_down(self, scalar: Self::Times) -> Option<Self>;

    /// Converts `self` into its underlying primitive representation.
    ///
    /// This is analogous to `impl Into<Times> for Self`.
    fn into_times(self) -> Self::Times;
}
