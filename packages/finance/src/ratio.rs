use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{fraction::Fraction, fractionable::Fractionable};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U> {
    fn parts(&self) -> U;
    fn total(&self) -> U;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U> {
    pub const fn new(nominator: U, denominator: U) -> Self {
        Self {
            nominator,
            denominator,
        }
    }
}

impl<U, T> Fraction<U> for Rational<T>
where
    Self: Ratio<U>,
{
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        whole.safe_mul(self)
    }
}

impl<U, T> Ratio<U> for Rational<T>
where
    T: Default + Copy + PartialEq + Into<U>,
{
    fn parts(&self) -> U {
        self.nominator.into()
    }

    fn total(&self) -> U {
        self.denominator.into()
    }
}
