use crate::{error::Error, Matcher, SymbolSlice};

use super::Currency;

pub trait SingleVisitor<C> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub type MaybeVisitResult<C, V> =
    Result<Result<<V as SingleVisitor<C>>::Output, <V as SingleVisitor<C>>::Error>, V>;

pub trait CurrencyVisit: Matcher {
    fn visit<C, V>(&self, symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        C: Currency,
        V: SingleVisitor<C>,
        Error: Into<V::Error>,
    {
        self.maybe_visit(symbol, visitor)
            .unwrap_or_else(|_| Err(Error::unexpected_symbol::<_, Self, C>(symbol).into()))
    }

    fn maybe_visit<C, V>(&self, ticker: &SymbolSlice, visitor: V) -> MaybeVisitResult<C, V>
    where
        C: Currency,
        V: SingleVisitor<C>,
    {
        if self.match_::<C>(ticker) {
            Ok(visitor.on())
        } else {
            Err(visitor)
        }
    }
}
impl<M> CurrencyVisit for M where M: Matcher {}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol::CurrencyVisit,
        test::{Expect, ExpectUnknownCurrency},
        test::{SuperGroupTestC1, SuperGroupTestC2},
        Currency,
    };
    use crate::{BankSymbols, Tickers};

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<SuperGroupTestC1>::default();
        Tickers
            .visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc.clone())
            .unwrap_err();
        assert_eq!(Tickers.visit(SuperGroupTestC1::TICKER, v_usdc), Ok(true));

        let v_nls = Expect::<SuperGroupTestC2>::default();
        assert_eq!(Tickers.visit(SuperGroupTestC2::TICKER, v_nls), Ok(true));
    }

    #[test]
    fn visit_on_ticker_unexpected() {
        const UNKNOWN_TICKER: &str = "my_fancy_coin";

        assert_eq!(
            Tickers.visit::<SuperGroupTestC2, _>(UNKNOWN_TICKER, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, Tickers, SuperGroupTestC2>(
                UNKNOWN_TICKER,
            )),
        );

        assert_eq!(
            Tickers.visit::<SuperGroupTestC2, _>(SuperGroupTestC1::TICKER, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, Tickers, SuperGroupTestC2>(
                SuperGroupTestC1::TICKER,
            )),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<SuperGroupTestC1>::default();
        assert_eq!(
            BankSymbols.visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2>::default();
        assert_eq!(
            BankSymbols.visit(SuperGroupTestC2::BANK_SYMBOL, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            BankSymbols.visit::<SuperGroupTestC2, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, BankSymbols, SuperGroupTestC2>(DENOM,)),
        );
    }
}
