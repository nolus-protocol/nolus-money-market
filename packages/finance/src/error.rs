use std::any::type_name;

use thiserror::Error;

use sdk::cosmwasm_std::{OverflowError, StdError};

use crate::currency::{Currency, Group, SymbolOwned};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Finance] Programming error or invalid serialized object of '{0}' type, cause '{1}'")]
    BrokenInvariant(String, String),

    #[error("[Finance] [OverflowError] {0}")]
    OverflowError(#[from] OverflowError),

    #[error("[Finance] Found ticker '{0}' expecting '{1}'")]
    UnexpectedTicker(String, String),

    #[error("[Finance] Found bank symbol '{0}' expecting '{1}'")]
    UnexpectedBankSymbol(String, String),

    #[error("[Finance] Found currency '{0}' which is not defined in the {1} currency group")]
    NotInCurrencyGroup(String, String),

    #[error("[Finance] Expecting funds of '{0}' but found none")]
    NoFunds(String),

    #[error("[Finance] Expecting funds of '{0}' but found extra ones")]
    UnexpectedFunds(String),

    #[error("[Finance] [Std] {0}")]
    CosmWasmError(#[from] StdError),
}

impl Error {
    pub fn broken_invariant_err<T>(msg: &str) -> Self {
        Self::BrokenInvariant(type_name::<T>().into(), msg.into())
    }

    pub fn no_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::NoFunds(C::TICKER.into())
    }

    pub fn unexpected_funds<C>() -> Self
    where
        C: Currency,
    {
        Self::UnexpectedFunds(C::TICKER.into())
    }

    pub fn unexpected_ticker<S, C>(ticker: S) -> Self
    where
        S: Into<SymbolOwned>,
        C: Currency,
    {
        Self::UnexpectedTicker(ticker.into(), C::TICKER.into())
    }

    pub fn unexpected_bank_symbol<S, C>(bank_symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        C: Currency,
    {
        Self::UnexpectedBankSymbol(bank_symbol.into(), C::BANK_SYMBOL.into())
    }

    pub fn not_in_currency_group<S, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), G::DESCR.into())
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

        let err = Error::broken_invariant_err::<TestX>(CAUSE);
        assert_eq!(
            &Error::BrokenInvariant(test_x_type_name.into(), CAUSE.into()),
            &err
        );

        assert_eq!(
            format!("{}", err),
            format!(
                "[Finance] Programming error or invalid serialized object of '{0}' type, cause '{1}'",
                test_x_type_name, CAUSE
            )
        );
    }
}
