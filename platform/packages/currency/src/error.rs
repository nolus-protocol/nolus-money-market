use thiserror::Error;

use crate::{
    Currency, CurrencyDTO, Definition, Group, MemberOf, Symbol, SymbolOwned, SymbolStatic,
};

#[derive(Error, Debug, PartialEq)]
pub enum Error {
    #[error("[Currency] Found a symbol '{0}' pretending to be the {1} of the currency with ticker '{2}'")]
    UnexpectedSymbol(String, String, String),

    #[error("[Currency] Found a symbol '{0}' pretending to be {1} of a currency pertaining to the {2} group")]
    NotInCurrencyGroup(String, String, String),

    #[error("[Currency] Expected currency {expected}, found {found}")]
    CurrencyMismatch {
        expected: String,
        found: SymbolStatic,
    },
}

impl Error {
    pub fn unexpected_symbol<S, CS, CDef>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
        CDef: Definition,
    {
        Self::UnexpectedSymbol(symbol.into(), CS::DESCR.into(), CDef::TICKER.into())
    }

    pub fn not_in_currency_group<S, CS, G>(symbol: S) -> Self
    where
        S: Into<SymbolOwned>,
        CS: Symbol + ?Sized,
        G: Group,
    {
        Self::NotInCurrencyGroup(symbol.into(), CS::DESCR.into(), G::DESCR.into())
    }

    pub fn currency_mismatch<C, G>(expected: &CurrencyDTO<G>) -> Self
    where
        C: Currency + MemberOf<G>,
        G: Group,
    {
        Self::CurrencyMismatch {
            expected: expected.to_string(),
            found: crate::to_string::<C>(),
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;
