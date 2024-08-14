use crate::{
    error::Error,
    matcher::{self, Matcher},
    symbol::Symbol,
    CurrencyDef, MemberOf, SymbolSlice,
};

pub trait SingleVisitor<CDef> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub trait CurrencyVisit: Symbol {
    //TODO check if we can remove the `CDef` type arg and pass &Definition
    fn visit<CDef, V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        CDef: CurrencyDef,
        CDef::Group: MemberOf<Self::Group>,
        V: SingleVisitor<CDef>,
        Error: Into<V::Error>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        let def = CDef::definition().dto().definition();
        if matcher.r#match(def) {
            visitor.on()
        } else {
            Err(Error::unexpected_symbol::<_, Self>(symbol, def).into())
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
            Expect, ExpectUnknownCurrency, SuperGroup, SuperGroupTestC2, TESTC1, TESTC1_DEFINITION,
            TESTC2, TESTC2_DEFINITION,
        },
    };
    use crate::{BankSymbols, Tickers};

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<_, SuperGroup, SuperGroup>::new(&TESTC1);
        Tickers::<SuperGroup>::visit(TESTC1_DEFINITION.bank_symbol, v_usdc.clone()).unwrap_err();
        assert_eq!(
            Tickers::<SuperGroup>::visit(TESTC1_DEFINITION.ticker, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<_, SuperGroup, SuperGroup>::new(&TESTC2);
        assert_eq!(
            Tickers::<SuperGroup>::visit(TESTC2_DEFINITION.ticker, v_nls),
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
            Err(Error::unexpected_symbol::<_, Tickers<SuperGroup>>(
                UNKNOWN_TICKER,
                &TESTC2_DEFINITION,
            )),
        );

        assert_eq!(
            Tickers::<SuperGroup>::visit::<SuperGroupTestC2, _>(
                TESTC1_DEFINITION.ticker,
                ExpectUnknownCurrency::<SuperGroup>::new()
            ),
            Err(Error::unexpected_symbol::<_, Tickers<SuperGroup>>(
                TESTC1_DEFINITION.ticker,
                &TESTC2_DEFINITION,
            )),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<_, SuperGroup, SuperGroup>::new(&TESTC1);
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(TESTC1_DEFINITION.bank_symbol, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<_, SuperGroup, SuperGroup>::new(&TESTC2);
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(TESTC2_DEFINITION.bank_symbol, v_nls),
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
            Err(Error::unexpected_symbol::<_, BankSymbols::<SuperGroup>>(
                DENOM,
                &TESTC2_DEFINITION,
            )),
        );
    }
}
