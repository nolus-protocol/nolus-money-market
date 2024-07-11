use crate::{error::Error, AnyVisitor, AnyVisitorResult, Currency, SymbolStatic};

pub trait Symbol {
    const DESCR: &'static str;

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency;
}

#[derive(Clone, Copy)]
pub struct Tickers;
impl Symbol for Tickers {
    const DESCR: &'static str = "ticker";

    fn symbol<CD>() -> SymbolStatic
    where
        CD: Currency,
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
        CD: Currency,
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
        CD: Currency,
    {
        CD::DEX_SYMBOL
    }
}

impl<T> AnyVisitor for T
where
    T: Symbol,
{
    type Output = SymbolStatic;
    type Error = Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        Ok(<Self as Symbol>::symbol::<C>())
    }
}
