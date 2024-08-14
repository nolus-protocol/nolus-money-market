use thiserror::Error;

use crate::{CurrencyDTO, Definition, Group, MemberOf, Symbol, SymbolOwned, SymbolStatic, Tickers};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found a symbol '{0}' pretending to be the {1} of the currency with ticker '{2}'")]
    UnexpectedSymbol(String, &'static str, SymbolStatic),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, &'static str, &'static str),

    #[error("[Currency] Expected currency {expected}, found {found}")]
    CurrencyMismatch {
        expected: SymbolStatic,
        found: SymbolStatic,
    },
}

impl Error {
    pub fn unexpected_symbol<S, CS>(symbol: S, def: &Definition) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
    {
        Self::UnexpectedSymbol(symbol.into(), CS::DESCR, def.ticker)
    }

    pub fn not_in_currency_group<S, CS, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), CS::DESCR, G::DESCR)
    }

    pub fn currency_mismatch<G, SubG>(expected: &CurrencyDTO<G>, found: &CurrencyDTO<SubG>) -> Self
    where
        G: Group,
        SubG: Group + MemberOf<G>,
    {
        Self::CurrencyMismatch {
            expected: expected.into_symbol::<Tickers<G>>(),
            found: found.into_symbol::<Tickers<G>>(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
