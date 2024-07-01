use crate::{
    error::Error,
    matcher::{self, Matcher, Symbol},
    Definition, SymbolSlice,
};

use super::Currency;

pub trait SingleVisitor<C> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub trait CurrencyVisit: Symbol {
    fn visit<C, V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        C: Currency + Definition,
        V: SingleVisitor<C>,
        Error: Into<V::Error>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        if matcher.match_::<C>() {
            visitor.on()
        } else {
            Err(Error::unexpected_symbol::<_, Self, C>(symbol).into())
        }
    }
}
impl<T> CurrencyVisit for T where T: Symbol {}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol::CurrencyVisit,
        test::{
            Expect, ExpectUnknownCurrency, SuperGroupCurrency, SuperGroupTestC1, SuperGroupTestC2,
        },
        Definition,
    };
    use crate::{BankSymbols, Tickers};

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroupCurrency>::default();
        Tickers::visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc.clone()).unwrap_err();
        assert_eq!(Tickers::visit(SuperGroupTestC1::TICKER, v_usdc), Ok(true));

        let v_nls = Expect::<SuperGroupTestC2, SuperGroupCurrency>::default();
        assert_eq!(Tickers::visit(SuperGroupTestC2::TICKER, v_nls), Ok(true));
    }

    #[test]
    fn visit_on_ticker_unexpected() {
        const UNKNOWN_TICKER: &str = "my_fancy_coin";

        assert_eq!(
            Tickers::visit::<SuperGroupTestC2, _>(
                UNKNOWN_TICKER,
                ExpectUnknownCurrency::<SuperGroupCurrency>::default()
            ),
            Err(Error::unexpected_symbol::<_, Tickers, SuperGroupTestC2>(
                UNKNOWN_TICKER,
            )),
        );

        assert_eq!(
            Tickers::visit::<SuperGroupTestC2, _>(
                SuperGroupTestC1::TICKER,
                ExpectUnknownCurrency::<SuperGroupCurrency>::default()
            ),
            Err(Error::unexpected_symbol::<_, Tickers, SuperGroupTestC2>(
                SuperGroupTestC1::TICKER,
            )),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroupCurrency>::default();
        assert_eq!(
            BankSymbols::visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroupCurrency>::default();
        assert_eq!(
            BankSymbols::visit(SuperGroupTestC2::BANK_SYMBOL, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            BankSymbols::visit::<SuperGroupTestC2, _>(
                DENOM,
                ExpectUnknownCurrency::<SuperGroupCurrency>::default()
            ),
            Err(Error::unexpected_symbol::<_, BankSymbols, SuperGroupTestC2>(DENOM,)),
        );
    }
}
