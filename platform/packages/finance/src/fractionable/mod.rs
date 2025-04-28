use std::{
    fmt::Debug,
    ops::{Div, Mul},
};

use crate::{fraction::Fraction, percent::Units as PercentUnits, zero::Zero};

mod coin;
mod duration;
mod percent;
mod price;
mod usize;

pub trait Fractionable<U> {
    #[track_caller]
    fn safe_mul<F>(self, fraction: &F) -> Self
    where
        F: Fraction<U>;
}

// TODO revisit its usability
pub trait Percentable: Fractionable<PercentUnits> {}

pub trait HigherRank<T> {
    type Type;
    // An intermediate type to handle cases when there is no TryInto<Self> for HigherRank::Type but
    // instead there is TryInto<HigherRank::Intermediate> for HigherRank::Type, and Into<Self> for HigherRank::Intermediate
    type Intermediate;
}

impl<T, D, DIntermediate, U> Fractionable<U> for T
where
    T: HigherRank<U, Type = D, Intermediate = DIntermediate> + Into<D>,
    D: TryInto<DIntermediate>,
    <D as TryInto<DIntermediate>>::Error: Debug,
    DIntermediate: Into<T>,
    D: Div<D, Output = D> + Mul<D, Output = D>,
    U: Copy + Debug + Into<D> + PartialEq + Ord + Zero,
{
    #[track_caller]
    fn safe_mul<F>(self, ratio: &F) -> Self
    where
        F: Fraction<U>,
    {
        // TODO debug_assert_eq!(T::BITS * 2, D::BITS);

        let parts = ratio.parts();
        let total = ratio.total();

        debug_assert!(parts <= total, "Fraction broken invariant: parts > total");

        if parts == total {
            self
        } else {
            let res_double: D = self.into() * parts.into();
            let res_double = res_double / total.into();
            let res_intermediate: DIntermediate =
                res_double.try_into().expect("unexpected overflow");
            res_intermediate.into()
        }
    }
}

impl<T> Percentable for T where T: Fractionable<PercentUnits> {}
