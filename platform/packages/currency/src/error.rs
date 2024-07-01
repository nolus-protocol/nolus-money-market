use thiserror::Error;

use crate::{symbol::Symbol, Definition, Group, SymbolOwned};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found a symbol '{0}' pretending to be the {1} of the currency with ticker '{2}'")]
    UnexpectedSymbol(String, String, String),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, String, String),
}

impl Error {
    pub(crate) fn unexpected_symbol<S, CS, SS>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
        SS: Definition,
    {
        Self::UnexpectedSymbol(symbol.into(), CS::DESCR.into(), SS::TICKER.into())
    }

    pub(crate) fn not_in_currency_group<S, CS, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), CS::DESCR.into(), G::DESCR.into())
    }
}

pub type Result<T> = core::result::Result<T, Error>;
