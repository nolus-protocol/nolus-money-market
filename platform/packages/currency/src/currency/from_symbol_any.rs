use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, Matcher, MaybeAnyVisitResult, SymbolSlice};

use super::{matcher::Tickers, Currency, Group};

use self::impl_any_tickers::FirstTickerVisitor;

pub type AnyVisitorResult<V> = Result<<V as AnyVisitor>::Output, <V as AnyVisitor>::Error>;
pub type AnyVisitorPairResult<V> =
    Result<<V as AnyVisitorPair>::Output, <V as AnyVisitorPair>::Error>;

pub trait AnyVisitor {
    type Output;
    type Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned;
}
pub trait AnyVisitorPair {
    type Output;
    type Error;

    fn on<C1, C2>(self) -> AnyVisitorPairResult<Self>
    where
        C1: Currency + Serialize + DeserializeOwned,
        C2: Currency + Serialize + DeserializeOwned;
}

pub trait GroupVisit: Matcher {
    fn visit_any<G, V>(&self, ticker: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        G: Group,
        V: AnyVisitor,
        Error: Into<V::Error>,
    {
        self.maybe_visit_any::<G, _>(ticker, visitor)
            .unwrap_or_else(|_| Err(Error::not_in_currency_group::<_, Self, G>(ticker).into()))
    }

    fn maybe_visit_any<G, V>(&self, ticker: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        G: Group,
        V: AnyVisitor,
    {
        G::maybe_visit(self, ticker, visitor)
    }
}
impl<M> GroupVisit for M where M: Matcher {}

pub fn visit_any_on_tickers<G1, G2, V>(
    ticker1: &SymbolSlice,
    ticker2: &SymbolSlice,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G1: Group,
    G2: Group,
    V: AnyVisitorPair,
    Error: Into<V::Error>,
{
    Tickers.visit_any::<G1, _>(ticker1, FirstTickerVisitor::<G2, _>::new(ticker2, visitor))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use serde::{de::DeserializeOwned, Serialize};

    use crate::{
        currency::{matcher::Tickers, Currency, Group, SymbolSlice},
        error::Error,
    };

    use super::{AnyVisitor, AnyVisitorPair, AnyVisitorResult, GroupVisit};

    pub struct FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        ticker2: &'a SymbolSlice,
        group2: PhantomData<G2>,
        visitor: V,
    }
    impl<'a, G2, V> FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: &'a SymbolSlice, visitor: V) -> Self {
            Self {
                ticker2,
                group2: PhantomData::<G2>,
                visitor,
            }
        }
    }
    impl<'a, G2, V> AnyVisitor for FirstTickerVisitor<'a, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
        Error: Into<V::Error>,
    {
        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self) -> AnyVisitorResult<Self>
        where
            C1: Currency + Serialize + DeserializeOwned,
        {
            Tickers.visit_any::<G2, _>(
                self.ticker2,
                SecondTickerVisitor {
                    currency1: PhantomData::<C1>,
                    visitor: self.visitor,
                },
            )
        }
    }

    struct SecondTickerVisitor<C1, V>
    where
        C1: Currency,
        V: AnyVisitorPair,
    {
        currency1: PhantomData<C1>,
        visitor: V,
    }
    impl<C1, V> AnyVisitor for SecondTickerVisitor<C1, V>
    where
        C1: Currency + Serialize + DeserializeOwned,
        V: AnyVisitorPair,
    {
        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self) -> AnyVisitorResult<Self>
        where
            C2: Currency + Serialize + DeserializeOwned,
        {
            self.visitor.on::<C1, C2>()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        currency::{
            from_symbol_any::GroupVisit,
            matcher::Tickers,
            test::{Expect, ExpectPair, ExpectUnknownCurrency},
            Currency, Group,
        },
        error::Error,
        test::{SubGroup, SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<SuperGroupTestC1>::default();
        assert_eq!(
            Ok(true),
            Tickers.visit_any::<SuperGroup, _>(SuperGroupTestC1::TICKER, v_usdc)
        );

        let v_nls = Expect::<SuperGroupTestC2>::default();
        assert_eq!(
            Ok(true),
            Tickers.visit_any::<SuperGroup, _>(SuperGroupTestC2::TICKER, v_nls)
        );

        assert_eq!(
            Err(Error::not_in_currency_group::<_, Tickers, SuperGroup>(
                SubGroupTestC1::BANK_SYMBOL
            )),
            Tickers.visit_any::<SuperGroup, _>(SubGroupTestC1::BANK_SYMBOL, ExpectUnknownCurrency)
        );
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            Tickers.visit_any::<SuperGroup, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::not_in_currency_group::<_, Tickers, SuperGroup>(
                DENOM
            )),
        );
    }

    #[test]
    fn visit_any_tickers() {
        visit_any_tickers_ok::<SuperGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_tickers_ok::<SubGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC1>();
        visit_any_tickers_ok::<SuperGroup, SuperGroup, SuperGroupTestC2, SuperGroupTestC2>();

        visit_any_tickers_fail::<SuperGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC2>();
    }

    fn visit_any_tickers_ok<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: 'static + Currency,
        C2: 'static + Currency,
    {
        let v_c1_c2 = ExpectPair::<C1, C2>::default();
        assert_eq!(
            Ok(true),
            super::visit_any_on_tickers::<G1, G2, _>(C1::TICKER, C2::TICKER, v_c1_c2)
        );
    }

    fn visit_any_tickers_fail<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: 'static + Currency,
        C2: 'static + Currency,
    {
        let v_c1_c2 = ExpectPair::<C1, C2>::default();
        assert!(super::visit_any_on_tickers::<G1, G2, _>(C1::TICKER, C2::TICKER, v_c1_c2).is_err());
    }
}
