use std::fmt::Display;

use crate::{error::Result, fractionable::Fractionable};

pub trait Fraction<U> {
    fn of<A>(&self, whole: A) -> Result<A>
    where
        A: Fractionable<U> + Display + Clone;
}
