use std::{any::type_name, fmt::Debug};

use thiserror::Error;

use currency::error::Error as CurrencyError;

use crate::percent::{Units as PercentUnits, permilles::Permilles};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Finance] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),

    #[error("[Finance] Fraction multiplication overflow when evaluating `{details}`")]
    MultiplicationOverflow { details: String },

    #[error("[Finance] {0}")]
    CurrencyError(#[from] CurrencyError),

    #[error(
        "[Finance] [Percent] Upper bound has been crossed! Upper bound is: {bound}, but got: {value}!"
    )]
    UpperBoundCrossed {
        bound: PercentUnits,
        value: Permilles,
    },
}

impl Error {
    pub fn broken_invariant_if<T>(check: bool, msg: &str) -> Result<()> {
        if check {
            Err(Self::BrokenInvariant(type_name::<T>().into(), msg.into()))
        } else {
            Ok(())
        }
    }

    pub fn multiplication_overflow<L, R>(lhs: L, rhs: R) -> Self
    where
        L: Debug,
        R: Debug,
    {
        Self::MultiplicationOverflow {
            details: format!("({:?}.of({:?}))", lhs, rhs),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use std::any::type_name;

    use super::Error;

    #[test]
    fn broken_invariant_err() {
        enum TestX {}
        let test_x_type_name: &str = type_name::<TestX>();
        const CAUSE: &str = "TestX failed";

        let err = Error::broken_invariant_if::<TestX>(true, CAUSE).expect_err("unexpected result");
        assert_eq!(
            &Error::BrokenInvariant(test_x_type_name.into(), CAUSE.into()),
            &err
        );

        assert_eq!(
            format!("{err}"),
            format!(
                "[Finance] Programming error or invalid serialized object of '{0}' type, cause '{1}'",
                test_x_type_name, CAUSE
            )
        );
    }
}
