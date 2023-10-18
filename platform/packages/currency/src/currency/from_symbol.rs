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
    use crate::currency::from_symbol::CurrencyVisit;
    use crate::test::{Nls, Usdc};
    use crate::{
        currency::Currency,
        error::Error,
        test::visitor::{Expect, ExpectUnknownCurrency},
    };
    use crate::{BankSymbols, Tickers};

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<Usdc>::default();
        Tickers
            .visit(Usdc::BANK_SYMBOL, v_usdc.clone())
            .unwrap_err();
        assert_eq!(Tickers.visit(Usdc::TICKER, v_usdc), Ok(true));

        let v_nls = Expect::<Nls>::default();
        assert_eq!(Tickers.visit(Nls::TICKER, v_nls), Ok(true));
    }

    #[test]
    fn visit_on_ticker_unexpected() {
        const UNKNOWN_TICKER: &str = "my_fancy_coin";

        assert_eq!(
            Tickers.visit::<Nls, _>(UNKNOWN_TICKER, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, Tickers, Nls>(UNKNOWN_TICKER,)),
        );

        assert_eq!(
            Tickers.visit::<Nls, _>(Usdc::TICKER, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, Tickers, Nls>(Usdc::TICKER,)),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<Usdc>::default();
        assert_eq!(BankSymbols.visit(Usdc::BANK_SYMBOL, v_usdc), Ok(true));

        let v_nls = Expect::<Nls>::default();
        assert_eq!(BankSymbols.visit(Nls::BANK_SYMBOL, v_nls), Ok(true));
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            BankSymbols.visit::<Nls, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::unexpected_symbol::<_, BankSymbols, Nls>(DENOM,)),
        );
    }
}
