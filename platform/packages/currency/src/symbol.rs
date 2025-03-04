use std::marker::PhantomData;

use crate::{Group, SymbolStatic, definition::Definition};

pub trait Symbol {
    const DESCR: &'static str;

    // type Group: Group + InPoolWith<Self::Group>;
    type Group: Group;
    type Symbol<SubG>: Symbol<Group = SubG>
    where
        SubG: Group;
    // SubG: Group + InPoolWith<SubG>;

    fn symbol(def: &Definition) -> SymbolStatic;
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

    type Symbol<SubG>
        = Tickers<SubG>
    where
        SubG: Group;

    fn symbol(def: &Definition) -> SymbolStatic {
        def.ticker
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

    type Symbol<SubG>
        = BankSymbols<SubG>
    where
        SubG: Group;

    fn symbol(def: &Definition) -> SymbolStatic {
        def.bank_symbol
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

    type Symbol<SubG>
        = DexSymbols<SubG>
    where
        SubG: Group;

    fn symbol(def: &Definition) -> SymbolStatic {
        def.dex_symbol
    }
}
