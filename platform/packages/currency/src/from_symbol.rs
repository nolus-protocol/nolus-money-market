use crate::{
    CurrencyDef,
    error::Error,
    matcher::{self, Matcher},
    symbol::Symbol,
};

pub trait SingleVisitor<CDef> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub trait CurrencyVisit: Symbol {
    //TODO check if we can remove the `CDef` type arg and pass &Definition
    fn visit<CDef, V>(symbol: &str, visitor: V) -> Result<V::Output, V::Error>
    where
        CDef: CurrencyDef,
        V: SingleVisitor<CDef>,
        Error: Into<V::Error>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        let def = CDef::dto().definition();
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
    use crate::{BankSymbols, Tickers};
    use crate::{
        CurrencyDef,
        error::Error,
        from_symbol::CurrencyVisit,
        test::{Expect, ExpectUnknownCurrency, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
    };

    #[test]
    fn visit_on_ticker() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        Tickers::<SuperGroup>::visit(SuperGroupTestC1::bank(), v_usdc.clone()).unwrap_err();
        assert_eq!(
            Tickers::<SuperGroup>::visit(SuperGroupTestC1::ticker(), v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Tickers::<SuperGroup>::visit(SuperGroupTestC2::ticker(), v_nls),
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
                SuperGroupTestC2::dto().definition()
            )),
        );

        assert_eq!(
            Tickers::<SuperGroup>::visit::<SuperGroupTestC2, _>(
                SuperGroupTestC1::ticker(),
                ExpectUnknownCurrency::<SuperGroup>::new()
            ),
            Err(Error::unexpected_symbol::<_, Tickers<SuperGroup>>(
                SuperGroupTestC1::ticker(),
                SuperGroupTestC2::dto().definition(),
            )),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(SuperGroupTestC1::bank(), v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::new();
        assert_eq!(
            BankSymbols::<SuperGroup>::visit(SuperGroupTestC2::bank(), v_nls),
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
                SuperGroupTestC2::dto().definition()
            )),
        );
    }
}
