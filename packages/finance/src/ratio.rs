use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{fraction::Fraction, fractionable::Fractionable};

pub trait Ratio<U>
where
    Self: Sized + Fraction<U>,
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

impl<U> Fraction<U> for Rational<U>
where
    U: Default + PartialEq + Copy,
{
    fn of<A>(&self, whole: A) -> A
    where
        A: Fractionable<U>,
    {
        whole.safe_mul(self)
    }
}

impl<U> Ratio<U> for Rational<U>
where
    U: Default + PartialEq + Copy,
{
    type Inv = Self;

    fn parts(&self) -> U {
        self.nominator
    }

    fn total(&self) -> U {
        self.denominator
    }

    fn inv(&self) -> Option<Self::Inv> {
        if self.nominator == U::default() {
            None
        } else {
            Some(Self {
                nominator: self.denominator,
                denominator: self.nominator,
            })
        }
    }
}
