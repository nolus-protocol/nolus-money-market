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

pub trait CurrencySymbol {
    const DESCR: &'static str;

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
    const DESCR: &'static str = "ticker";

    type Symbol<C> = Ticker<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub struct BankSymbolMatcher;
impl CurrencySymbol for BankSymbolMatcher {
    const DESCR: &'static str = "bank symbol";

    type Symbol<C> = BankSymbol<C> where C: Currency;
}

#[derive(Clone, Copy)]
pub struct DexSymbolMatcher;
impl CurrencySymbol for DexSymbolMatcher {
    const DESCR: &'static str = "dex symbol";

    type Symbol<C: Currency> = DexSymbol<C> where C: Currency;
}
