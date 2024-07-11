use std::fmt::Debug;

use currency::{test::Expect, BankSymbols, Currency, Group, GroupVisit, SymbolSlice, Tickers};

#[track_caller]
pub fn maybe_visit_on_ticker_impl<C, G>()
where
    C: 'static + Currency,
    G: Group,
{
    let v = Expect::<C>::default();
    assert_eq!(Tickers::maybe_visit_any::<G, _>(C::TICKER, v), Ok(Ok(true)));
}

#[track_caller]
pub fn maybe_visit_on_ticker_err<C, G>(unknown_ticker: &SymbolSlice)
where
    C: 'static + Clone + Debug + PartialEq,
    G: Group,
{
    let v = Expect::<C>::default();
    assert_eq!(
        Tickers::maybe_visit_any::<G, _>(unknown_ticker, v.clone()),
        Err(v)
    );
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_impl<C, G>()
where
    C: Currency,
    G: Group,
{
    let v = Expect::<C>::default();
    assert_eq!(
        BankSymbols::maybe_visit_any::<G, _>(C::BANK_SYMBOL, v),
        Ok(Ok(true))
    );
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_err<C, G>(unknown_ticker: &SymbolSlice)
where
    C: Currency,
    G: Group,
{
    let v = Expect::<C>::default();
    assert_eq!(
        BankSymbols::maybe_visit_any::<G, _>(unknown_ticker, v.clone()),
        Err(v)
    );
}
