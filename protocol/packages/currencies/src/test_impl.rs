use currency::{
    error::Error, test::Expect, BankSymbols, CurrencyDef, Group, GroupVisit, MemberOf, Symbol,
    Tickers,
};

#[track_caller]
pub fn maybe_visit_on_ticker_impl<C, VisitorG>()
where
    C: CurrencyDef,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol::<C, VisitorG, Tickers<C::Group>>(C::ticker())
}

#[track_caller]
pub fn maybe_visit_on_ticker_err<C, VisitorG>(unknown_ticker: &str)
where
    C: CurrencyDef,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol_err::<C, VisitorG, Tickers<C::Group>>(unknown_ticker)
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_impl<C, VisitorG>()
where
    C: CurrencyDef,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol::<C, VisitorG, BankSymbols<C::Group>>(C::bank())
}

#[track_caller]
pub fn maybe_visit_on_bank_symbol_err<C, VisitorG>(unknown_ticker: &str)
where
    C: CurrencyDef,
    C::Group: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    visit_on_symbol_err::<C, VisitorG, BankSymbols<C::Group>>(unknown_ticker)
}

fn visit_on_symbol<C, VisitorG, Symbols>(symbol: &str)
where
    C: CurrencyDef,
    C::Group: MemberOf<VisitorG>,
    VisitorG: Group,
    Symbols: Symbol<Group = C::Group>,
{
    assert_eq!(
        Symbols::visit_any(symbol, Expect::<C, C::Group, C::Group>::default()),
        Ok(true)
    );
}

fn visit_on_symbol_err<C, VisitorG, Symbols>(unknown_symbol: &str)
where
    C: CurrencyDef,
    C::Group: MemberOf<VisitorG>,
    VisitorG: Group,
    Symbols: Symbol<Group = C::Group>,
{
    assert!(matches!(
        Symbols::visit_any(unknown_symbol, Expect::<C, C::Group, C::Group>::default()),
        Err(Error::NotInCurrencyGroup(_, _, _))
    ));
}
