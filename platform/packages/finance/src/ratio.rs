use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    error::{Error, Result as FinanceResult},
    fraction::Fraction,
    fractionable::Fractionable,
    zero::Zero,
};

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

impl<U> Rational<U>
where
    U: Zero + Debug + PartialEq<U>,
{
    #[track_caller]
    pub fn new(nominator: U, denominator: U) -> Self {
        debug_assert_ne!(denominator, Zero::ZERO);

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
    #[track_caller]
    fn of<A>(&self, whole: A) -> FinanceResult<A>
    where
        A: Fractionable<U>,
    {
        whole.checked_mul(self).ok_or(Error::MultiplicationOverflow(
            "Overflow during rational multiplication",
        ))
    }
}

impl<U, T> Ratio<U> for Rational<T>
where
    T: Zero + Copy + PartialEq + Into<U>,
{
    fn parts(&self) -> U {
        self.nominator.into()
    }

    fn total(&self) -> U {
        self.denominator.into()
    }
}
