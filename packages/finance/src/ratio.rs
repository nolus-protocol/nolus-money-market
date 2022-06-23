use cosmwasm_std::Fraction;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::fractionable::Fractionable;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Ratio<U> {
    nominator: U,
    denominator: U,
}

impl<U> Ratio<U> {
    pub fn new(nominator: U, denominator: U) -> Self {
        Self {
            nominator,
            denominator,
        }
    }
}

impl<U> Ratio<U>
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

impl<U> Fraction<U> for Ratio<U>
where
    U: Default + PartialEq + Copy,
{
    fn numerator(&self) -> U {
        self.nominator
    }

    fn denominator(&self) -> U {
        self.denominator
    }

    fn inv(&self) -> Option<Self> {
        if self.nominator == U::default() {
            None
        } else {
            Some(Ratio {
                nominator: self.denominator,
                denominator: self.nominator,
            })
        }
    }
}
