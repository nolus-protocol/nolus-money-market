use crate::{
    error::Error,
    matcher::{self, Matcher},
    symbol::Symbol,
    Definition, MemberOf, SymbolSlice,
};

pub trait SingleVisitor<CDef> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub trait CurrencyVisit: Symbol {
    fn visit<CDef, V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        CDef: Definition + MemberOf<Self::Group>,
        V: SingleVisitor<CDef>,
        Error: Into<V::Error>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        if matcher.r#match::<CDef>() {
            visitor.on()
        } else {
            Err(Error::unexpected_symbol::<_, Self, CDef>(symbol).into())
        }
    }
}
impl<T> CurrencyVisit for T where T: Symbol {}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol::CurrencyVisit,
        test::{Expect, ExpectUnknownCurrency, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        Definition,
    };
    use crate::{BankSymbols, Tickers};

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::default();
        Tickers::<SuperGroup>::visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc.clone()).unwrap_err();
        assert_eq!(
            Tickers::<SuperGroup>::visit(SuperGroupTestC1::TICKER, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::default();
        assert_eq!(
            Tickers::<SuperGroup>::visit(SuperGroupTestC2::TICKER, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_ticker_unexpected() {
        const UNKNOWN_TICKER: &str = "my_fancy_coin";

        assert_eq!(
            Tickers::<SuperGroup>::visit::<SuperGroupTestC2, _>(
                UNKNOWN_TICKER,
                ExpectUnknownCurrency::<SuperGroup>::new()
            ),
            Err(Error::unexpected_symbol::<
                _,
                Tickers<SuperGroup>,
                SuperGroupTestC2,
            >(UNKNOWN_TICKER,)),
        );

        assert_eq!(
            Tickers::<SuperGroup>::visit::<SuperGroupTestC2, _>(
                SuperGroupTestC1::TICKER,
                ExpectUnknownCurrency::<SuperGroup>::new()
            ),
            Err(Error::unexpected_symbol::<
                _,
                Tickers<SuperGroup>,
                SuperGroupTestC2,
            >(SuperGroupTestC1::TICKER,)),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::default();
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(SuperGroupTestC1::BANK_SYMBOL, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::default();
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(SuperGroupTestC2::BANK_SYMBOL, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            BankSymbols::<SuperGroup>::visit::<SuperGroupTestC2, _>(
                DENOM,
                ExpectUnknownCurrency::<SuperGroup>::new()
            ),
            Err(Error::unexpected_symbol::<
                _,
                BankSymbols::<SuperGroup>,
                SuperGroupTestC2,
            >(DENOM,)),
        );
    }
}
