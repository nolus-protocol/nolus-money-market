use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{fraction::Fraction, fractionable::Fractionable};

// TODO review whether it may gets simpler if extend Fraction
pub trait Ratio<U>
{
    type Inv: Ratio<U>;

    fn parts(&self) -> U;
    fn total(&self) -> U;
    fn inv(&self) -> Option<Self::Inv>;
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Rational<U> {
    nominator: U,
    denominator: U,
}

impl<U> Rational<U> {
    pub fn new(nominator: U, denominator: U) -> Self {
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
    type Inv = Self;

    fn parts(&self) -> U {
        self.nominator.into()
    }

    fn total(&self) -> U {
        self.denominator.into()
    }

    fn inv(&self) -> Option<Self::Inv> {
        if self.nominator == T::default() {
            None
        } else {
            Some(Self {
                nominator: self.denominator,
                denominator: self.nominator,
            })
        }
    }
}
