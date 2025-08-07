use std::fmt::Debug;

use crate::{
    fractionable::{Fractionable, ToPrimitive},
    zero::Zero,
};

pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
        U: ToPrimitive<A::HigherPrimitive>;
}

pub trait Unit
where
    Self: Copy + Debug + Ord + Zero,
{
}
