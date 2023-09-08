use std::marker::PhantomData;

use crate::SymbolSlice;

use super::{Currency, SymbolStatic};

pub trait Symbol {
    const DESCRIPTION: &'static str;
    const VALUE: SymbolStatic;
}

pub struct Ticker<C>(PhantomData<C>);
impl<C> Symbol for Ticker<C>
where
    C: Currency,
{
    const DESCRIPTION: &'static str = "ticker";
    const VALUE: SymbolStatic = C::TICKER;
}

pub struct BankSymbol<C>(PhantomData<C>);
impl<C> Symbol for BankSymbol<C>
where
    C: Currency,
{
    const DESCRIPTION: &'static str = "bank symbol";
    const VALUE: SymbolStatic = C::BANK_SYMBOL;
}

pub struct DexSymbol<C>(PhantomData<C>);
impl<C> Symbol for DexSymbol<C>
where
    C: Currency,
{
    const DESCRIPTION: &'static str = "dex symbol";
    const VALUE: SymbolStatic = C::DEX_SYMBOL;
}

pub trait CurrencySymbol {
    type Symbol<C>: Symbol
    where
        C: Currency;
}

pub trait Matcher: CurrencySymbol {
    fn match_<C>(&self, field_value: &SymbolSlice) -> bool
    where
        C: Currency,
    {
        field_value == <Self::Symbol<C> as Symbol>::VALUE
    }
}

impl<T> Matcher for T where T: CurrencySymbol + ?Sized + Copy {}

#[derive(Clone, Copy)]
pub struct TickerMatcher;
impl CurrencySymbol for TickerMatcher {
    type Symbol<C> = Ticker<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub struct BankSymbolMatcher;
impl CurrencySymbol for BankSymbolMatcher {
    type Symbol<C> = BankSymbol<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub struct DexSymbolMatcher;
impl CurrencySymbol for DexSymbolMatcher {
    type Symbol<C: Currency> = DexSymbol<C> where C: Currency;
}
