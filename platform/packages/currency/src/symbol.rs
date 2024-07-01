use crate::{Definition,  SymbolStatic};

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
        CD::DEX_SYMBOL
    }
}
