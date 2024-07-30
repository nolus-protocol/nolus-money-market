use currency::{
    error::Error, test::Expect, BankSymbols, Currency, Group, GroupVisit, MemberOf, Symbol,
    SymbolSlice, Tickers,
};

#[track_caller]
pub fn maybe_visit_on_ticker_impl<C, VisitorG>()
where
    C: Currency,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol::<C, VisitorG, Tickers<C::Group>>(C::TICKER)
}

#[track_caller]
pub fn maybe_visit_on_ticker_err<C, VisitorG>(unknown_ticker: &SymbolSlice)
where
    C: Currency,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol_err::<C, VisitorG, Tickers<C::Group>>(unknown_ticker)
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_impl<C, VisitorG>()
where
    C: Currency,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol::<C, VisitorG, BankSymbols<C::Group>>(C::BANK_SYMBOL)
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_err<C, VisitorG>(unknown_ticker: &SymbolSlice)
where
    C: Currency,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol_err::<C, VisitorG, BankSymbols<C::Group>>(unknown_ticker)
}

fn visit_on_symbol<C, VisitorG, Symbols>(symbol: &SymbolSlice)
where
    C: Currency + MemberOf<C::Group>,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
    Symbols: Symbol<Group = C::Group>,
{
    assert_eq!(
        Symbols::visit_any(symbol, Expect::<C, C::Group, C::Group>::default()),
        Ok(true)
    );
    assert_eq!(
        Symbols::visit_member_any(symbol, Expect::<C, C::Group, VisitorG>::default()),
        Ok(true)
    );
    assert_eq!(
        Symbols::maybe_visit_member_any(symbol, Expect::<C, C::Group, VisitorG>::default()),
        Ok(Ok(true))
    );
}

fn visit_on_symbol_err<C, VisitorG, Symbols>(unknown_symbol: &SymbolSlice)
where
    C: Currency,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
    Symbols: Symbol<Group = C::Group>,
{
    assert!(matches!(
        Symbols::visit_any(unknown_symbol, Expect::<C, C::Group, C::Group>::default()),
        Err(Error::NotInCurrencyGroup(_, _, _))
    ));

    assert!(matches!(
        Symbols::visit_member_any(unknown_symbol, Expect::<C, C::Group, VisitorG>::default()),
        Err(Error::NotInCurrencyGroup(_, _, _))
    ));

    let v = Expect::<C, C::Group, VisitorG>::default();
    assert_eq!(
        Symbols::maybe_visit_member_any(unknown_symbol, v.clone()),
        Err(v)
    );
}
