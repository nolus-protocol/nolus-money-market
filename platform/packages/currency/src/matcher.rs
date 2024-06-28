use std::{any::TypeId, marker::PhantomData};

use crate::SymbolSlice;

use super::{Currency, SymbolStatic};

pub trait Symbols {
    /// Identifier of the currency
    const TICKER: SymbolStatic;

    /// Symbol at the Nolus network used by the Cosmos-SDK modules, mainly the Banking one
    const BANK_SYMBOL: SymbolStatic;

    /// Symbol at the Dex network
    const DEX_SYMBOL: SymbolStatic;

    const DECIMAL_DIGITS: u8;
}

pub trait Matcher {
    fn match_<C>(&self) -> bool
    where
        C: Currency + Symbols;
}

pub(crate) fn symbol_matcher<'a, S>(symbol: &'a SymbolSlice) -> impl Matcher + 'a
where
    S: 'a + Symbol + ?Sized,
{
    SymbolMatcher::<'a, S>(symbol, PhantomData)
}

struct SymbolMatcher<'a, S>(&'a SymbolSlice, PhantomData<S>)
where
    S: ?Sized;
impl<'a, S> Matcher for SymbolMatcher<'a, S>
where
    S: Symbol + ?Sized,
{
    fn match_<CS>(&self) -> bool
    where
        CS: Symbols,
    {
        self.0 == S::symbol::<CS>()
    }
}

#[derive(Debug)]
pub struct TypeMatcher(TypeId);
impl TypeMatcher {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<TypeId>,
    {
        Self(id.into())
    }
}
impl Matcher for TypeMatcher {
    fn match_<CS>(&self) -> bool
    where
        CS: 'static,
    {
        TypeId::of::<CS>() == self.0
    }
}

pub trait Symbol {
    const DESCR: &'static str;

    fn symbol<S>() -> SymbolStatic
    where
        S: Symbols;
}

#[derive(Clone, Copy)]
pub struct Tickers;
impl Symbol for Tickers {
    const DESCR: &'static str = "ticker";

    fn symbol<S>() -> SymbolStatic
    where
        S: Symbols,
    {
        S::TICKER
    }
}

#[derive(Clone, Copy)]
pub struct BankSymbols;
impl Symbol for BankSymbols {
    const DESCR: &'static str = "bank symbol";

    fn symbol<S>() -> SymbolStatic
    where
        S: Symbols,
    {
        S::BANK_SYMBOL
    }
}

#[derive(Clone, Copy)]
pub struct DexSymbols;
impl Symbol for DexSymbols {
    const DESCR: &'static str = "dex symbol";

    fn symbol<S>() -> SymbolStatic
    where
        S: Symbols,
    {
        S::BANK_SYMBOL
    }
}
