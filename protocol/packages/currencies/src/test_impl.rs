use currency::{
    group::MemberOf, test::Expect, BankSymbols, Currency, Group, GroupVisit, SymbolSlice, Symbols,
    Tickers,
};

#[track_caller]
pub fn maybe_visit_on_ticker_impl<C, G>()
where
    C: Currency + MemberOf<G> + Symbols,
    G: Group,
{
    let v = Expect::<C, G>::default();
    assert_eq!(Tickers::maybe_visit_any(C::TICKER, v), Ok(Ok(true)));
}

#[track_caller]
pub fn maybe_visit_on_ticker_err<C, G>(unknown_ticker: &SymbolSlice)
where
    C: Currency + MemberOf<G> + Symbols,
    G: Group,
{
    let v = Expect::<C, G>::default();
    assert_eq!(Tickers::maybe_visit_any(unknown_ticker, v.clone()), Err(v));
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_impl<C, G>()
where
    C: Currency + MemberOf<G> + Symbols,
    G: Group,
{
    let v = Expect::<C, G>::default();
    assert_eq!(
        BankSymbols::maybe_visit_any(C::BANK_SYMBOL, v),
        Ok(Ok(true))
    );
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_err<C, G>(unknown_ticker: &SymbolSlice)
where
    C: Currency + MemberOf<G> + Symbols,
    G: Group,
{
    let v = Expect::<C, G>::default();
    assert_eq!(
        BankSymbols::maybe_visit_any(unknown_ticker, v.clone()),
        Err(v)
    );
}
