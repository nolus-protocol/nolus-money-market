use std::marker::PhantomData;

use crate::SymbolSlice;

use super::{Currency, SymbolStatic};

pub trait Symbol {
    const VALUE: SymbolStatic;
}

pub struct Ticker<C>(PhantomData<C>);
impl<C> Symbol for Ticker<C>
where
    C: Currency,
{
    const VALUE: SymbolStatic = C::TICKER;
}

pub struct BankSymbol<C>(PhantomData<C>);
impl<C> Symbol for BankSymbol<C>
where
    C: Currency,
{
    const VALUE: SymbolStatic = C::BANK_SYMBOL;
}

pub struct DexSymbol<C>(PhantomData<C>);
impl<C> Symbol for DexSymbol<C>
where
    C: Currency,
{
    const VALUE: SymbolStatic = C::DEX_SYMBOL;
}

pub trait MatcherSpec {
    type Symbol<C>: Symbol
    where
        C: Currency;
}

pub trait Matcher: MatcherSpec + Copy {
    fn match_<C>(&self, field_value: &SymbolSlice) -> bool
    where
        C: Currency,
    {
        field_value == <Self::Symbol<C> as Symbol>::VALUE
    }
}

impl<T> Matcher for T where T: MatcherSpec + ?Sized + Copy {}

#[derive(Clone, Copy)]
pub(super) struct TickerMatcher;
impl MatcherSpec for TickerMatcher {
    type Symbol<C> = Ticker<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub(super) struct BankSymbolMatcher;
impl MatcherSpec for BankSymbolMatcher {
    type Symbol<C> = BankSymbol<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub(super) struct DexSymbolMatcher;
impl MatcherSpec for DexSymbolMatcher {
    type Symbol<C: Currency> = DexSymbol<C> where C: Currency;
}
