use std::any::type_name;

use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

use crate::currency::Currency;

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Finance] Programming error or invalid serialized object of {0} type")]
    BrokenInvariant(String),

    #[error("[Finance] [OverflowError] {0}")]
    OverflowError(#[from] OverflowError),

    #[error("[Finance] Found currency {0} expecting {1}")]
    UnexpectedCurrency(String, String),

    #[error("[Finance] Expecting funds of {0} but found none")]
    NoFunds(String),

    #[error("[Finance] Expecting funds of {0} but found extra ones")]
    UnexpectedFunds(String),

    #[error("[Finance] [Std] {0}")]
    CosmWasmError(#[from] StdError),
}

impl Error {
    pub fn broken_invariant_err<T>(msg: &str) -> Self {
        Self::BrokenInvariant(String::from(type_name::<T>()) + " => " + msg)
    }

    pub fn no_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::NoFunds(C::SYMBOL.into())
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::UnexpectedFunds(C::SYMBOL.into())
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

        let err = Error::broken_invariant_err::<TestX>("TestX failed");
        assert_eq!(
            &Error::BrokenInvariant(String::from(test_x_type_name) + " => TestX failed"),
            &err
        );

        assert_eq!(
            format!("{}", err),
            format!(
                "[Finance] Programming error or invalid serialized object of {} => TestX failed type",
                test_x_type_name
            )
        );
    }
}
