use std::marker::PhantomData;

use crate::{
    error::Error, group::MemberOf, AnyVisitor, AnyVisitorResult, Currency, Group, SymbolStatic,
};

pub trait Symbol {
    const DESCR: &'static str;

    type Group: Group;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency + MemberOf<Self::Group>;
}

#[derive(Clone, Copy, Default)]
pub struct Tickers<G> {
    group: PhantomData<G>,
}
impl<G> Tickers<G> {
    pub fn new() -> Self {
        Self { group: PhantomData }
    }
}
impl<G> Symbol for Tickers<G>
where
    G: Group,
{
    const DESCR: &'static str = "ticker";

    type Group = G;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency,
    {
        CD::TICKER
    }
}

#[derive(Clone, Copy, Default)]
pub struct BankSymbols<G> {
    group: PhantomData<G>,
}
impl<G> BankSymbols<G> {
    pub fn new() -> Self {
        Self { group: PhantomData }
    }
}
impl<G> Symbol for BankSymbols<G>
where
    G: Group,
{
    const DESCR: &'static str = "bank symbol";

    type Group = G;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency,
    {
        CD::BANK_SYMBOL
    }
}

#[derive(Clone, Copy, Default)]
pub struct DexSymbols<G> {
    group: PhantomData<G>,
}

impl<G> DexSymbols<G> {
    pub fn new() -> Self {
        Self { group: PhantomData }
    }
}
impl<G> Symbol for DexSymbols<G>
where
    G: Group,
{
    const DESCR: &'static str = "dex symbol";

    type Group = G;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency,
    {
        CD::DEX_SYMBOL
    }
}

impl<T> AnyVisitor for T
where
    T: Symbol,
{
    type VisitedG = T::Group;
    type Output = SymbolStatic;
    type Error = Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + MemberOf<Self::VisitedG>,
    {
        Ok(<Self as Symbol>::symbol::<C>())
    }
}
