use thiserror::Error;

use crate::{CurrencyDTO, Definition, Group, MemberOf, Symbol, SymbolOwned, SymbolStatic, Tickers};

// TODO replace SymbolStatic and SymbolOwned with CurrencyDTO<G> where approptiate, i.e. the string represent a currency
#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found a symbol '{0}' pretending to be the {1} of the currency with ticker '{2}'")]
    UnexpectedSymbol(String, &'static str, SymbolStatic),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, &'static str, &'static str),

    #[error("[Currency] No records for a pool with '{buddy1}' and '{buddy2}'")]
    NotInPoolWith {
        buddy1: SymbolStatic,
        buddy2: SymbolStatic,
    },

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

    pub fn currency_mismatch<ExpG, G>(expected: &CurrencyDTO<ExpG>, found: &CurrencyDTO<G>) -> Self
    where
        ExpG: Group + MemberOf<G>,
        G: Group,
    {
        Self::CurrencyMismatch {
            expected: expected.into_symbol::<Tickers<ExpG>>(),
            found: found.into_symbol::<Tickers<G>>(),
        }
    }

    pub fn not_in_pool_with<G>(c1: &CurrencyDTO<G>, c2: &CurrencyDTO<G>) -> Self
    where
        G: Group,
    {
        Self::NotInPoolWith {
            buddy1: c1.into_symbol::<Tickers<G>>(),
            buddy2: c2.into_symbol::<Tickers<G>>(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
