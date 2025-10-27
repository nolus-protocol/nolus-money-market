use std::ops::Div;

use crate::fractionable::checked_mul::CheckedMul;

pub(crate) mod checked_mul;

/// Converts the domain type into its wider primitive `Double` type
pub trait ToDoublePrimitive {
    type Double;

    fn to_double(&self) -> Self::Double;
}

/// Defines a common `Max` type, chosen as one of `Double` the types from either `Self` or `Other`
pub trait CommonDoublePrimitive<Other> {
    type CommonDouble: CheckedMul<Output = Self::CommonDouble> + Div<Output = Self::CommonDouble>;
}

/// Domain entity for which a fraction could be calculated.
pub trait Fractionable<FractionUnit>
where
    Self: CommonDoublePrimitive<FractionUnit>
        + TryFromMax<<Self as CommonDoublePrimitive<FractionUnit>>::CommonDouble>
        + Sized,
{
}

pub trait IntoMax<Max>
where
    Self: ToDoublePrimitive,
{
    fn into_max(self) -> Max;
}

/// Conversion from `Max` back to the domain type
pub trait TryFromMax<Max>
where
    Self: IntoMax<Max> + Sized,
{
    fn try_from_max(max: Max) -> Option<Self>;
}

pub trait HigherRank<T> {
    type Type;
}
