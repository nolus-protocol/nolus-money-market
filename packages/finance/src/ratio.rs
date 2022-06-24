use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::fractionable::Fractionable;

pub trait Ratio<U>
where
    Self: Sized,
{
    fn parts(&self) -> U;
    fn total(&self) -> U;
    fn inv(&self) -> Option<Self>;
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

impl<U> Rational<U>
where
    U: Default + PartialEq + Copy,
{
    pub fn of<A>(&self, amount: A) -> A
    where
        A: Fractionable<U>,
    {
        amount.safe_mul(self)
    }
}

impl<U> Ratio<U> for Rational<U>
where
    U: Default + PartialEq + Copy,
{
    fn parts(&self) -> U {
        self.nominator
    }

    fn total(&self) -> U {
        self.denominator
    }

    fn inv(&self) -> Option<Self> {
        if self.nominator == U::default() {
            None
        } else {
            Some(Rational {
                nominator: self.denominator,
                denominator: self.nominator,
            })
        }
    }
}
