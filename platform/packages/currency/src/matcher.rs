use std::{any::TypeId, marker::PhantomData};

use crate::{Definition, SymbolSlice};

use super::{Currency, SymbolStatic};

pub trait Matcher {
    fn r#match<C>(&self) -> bool
    where
        C: Currency + Definition;
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
    fn r#match<CD>(&self) -> bool
    where
        CD: Definition,
    {
        self.0 == S::symbol::<CD>()
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
    fn r#match<C>(&self) -> bool
    where
        C: 'static,
    {
        TypeId::of::<C>() == self.0
    }
}

pub trait Symbol {
    const DESCR: &'static str;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Definition;
}

#[derive(Clone, Copy)]
pub struct Tickers;
impl Symbol for Tickers {
    const DESCR: &'static str = "ticker";

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Definition,
    {
        CD::TICKER
    }
}

#[derive(Clone, Copy)]
pub struct BankSymbols;
impl Symbol for BankSymbols {
    const DESCR: &'static str = "bank symbol";

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Definition,
    {
        CD::BANK_SYMBOL
    }
}

#[derive(Clone, Copy)]
pub struct DexSymbols;
impl Symbol for DexSymbols {
    const DESCR: &'static str = "dex symbol";

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Definition,
    {
        CD::BANK_SYMBOL
    }
}
