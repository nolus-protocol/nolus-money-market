use std::ops::Div;

use crate::fractionable::checked_mul::CheckedMul;

pub(crate) mod checked_mul;

/// Defines a `CommonDouble` type, which is the [ToDoublePrimitive::Double] of either domain types: `Self` or `Other`.
///
/// `CommonDouble` is used as a wider buffer for performing multiplication opperations without an overflow.
pub trait CommonDoublePrimitive<Other> {
    type CommonDouble: CheckedMul<Output = Self::CommonDouble> + Div<Output = Self::CommonDouble>;
}

/// Domain entity for which a fraction could be calculated.
pub trait Fractionable<FractionUnit>
where
    Self: CommonDoublePrimitive<FractionUnit> + TryFromMax<Self::CommonDouble> + Sized,
{
}

/// Converts a domain type into `Max`
/// where `Max = <Self as CommonDoublePrimitive<Other>>::CommonDouble`
pub trait IntoMax<Max>
where
    Self: IntoDoublePrimitive,
{
    fn into_max(self) -> Max;
}

/// Converts a domain type into its wider primitive `Double` type
pub trait IntoDoublePrimitive {
    type Double;

    fn into_double(self) -> Self::Double;
}

/// Attempts to convert `Max` into the domain type `Self`
/// where `Max = <Self as CommonDoublePrimitive<Other>>::CommonDouble`
///
/// Returns [None] if the value cannot fit within the `Self` type
pub trait TryFromMax<Max>
where
    Self: IntoMax<Max> + Sized,
{
    fn try_from_max(max: Max) -> Option<Self>;
}
