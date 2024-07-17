use crate::{
    error::Error, group::MemberOf, matcher, MaybeAnyVisitResult, Symbol, SymbolSlice, Tickers,
};

use super::{Currency, Group};

use self::impl_any_tickers::FirstTickerVisitor;

pub type AnyVisitorResult<Visitor> =
    Result<<Visitor as AnyVisitor>::Output, <Visitor as AnyVisitor>::Error>;
pub type AnyVisitorPairResult<V> =
    Result<<V as AnyVisitorPair>::Output, <V as AnyVisitorPair>::Error>;

pub trait AnyVisitor {
    type VisitedG: Group;

    type Output;
    type Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency + MemberOf<Self::VisitedG>;
}
pub trait AnyVisitorPair {
    type VisitedG1: Group;
    type VisitedG2: Group;

    type Output;
    type Error;

    fn on<C1, C2>(self) -> AnyVisitorPairResult<Self>
    where
        C1: Currency + MemberOf<Self::VisitedG1>,
        C2: Currency + MemberOf<Self::VisitedG2>;
}

pub trait GroupVisit: Symbol {
    fn visit_any<V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<VisitedG = Self::Group>,
        Error: Into<V::Error>,
    {
        Self::maybe_visit_any(symbol, visitor).unwrap_or_else(|_| {
            Err(Error::not_in_currency_group::<_, Self, V::VisitedG>(symbol).into())
        })
    }

    fn maybe_visit_any<V>(symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<V>
    where
        V: AnyVisitor<VisitedG = Self::Group>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        V::VisitedG::maybe_visit(&matcher, visitor)
    }
}
impl<T> GroupVisit for T where T: Symbol {}

pub fn visit_any_on_tickers<G1, G2, V>(
    ticker1: &SymbolSlice,
    ticker2: &SymbolSlice,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G1: Group,
    G2: Group,
    V: AnyVisitorPair<VisitedG1 = G1, VisitedG2 = G2>,
    Error: Into<V::Error>,
{
    Tickers::<G1>::visit_any(
        ticker1,
        FirstTickerVisitor::<G1, G2, _>::new(ticker2, visitor),
    )
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use crate::{error::Error, Currency, Group, MemberOf, SymbolSlice, Tickers};

    use super::{AnyVisitor, AnyVisitorPair, AnyVisitorResult, GroupVisit};

    pub struct FirstTickerVisitor<'a, G1, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        group1: PhantomData<G1>,
        ticker2: &'a SymbolSlice,
        group2: PhantomData<G2>,
        visitor: V,
    }
    impl<'a, G1, G2, V> FirstTickerVisitor<'a, G1, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: &'a SymbolSlice, visitor: V) -> Self {
            Self {
                group1: PhantomData::<G1>,
                ticker2,
                group2: PhantomData::<G2>,
                visitor,
            }
        }
    }
    impl<'a, G1, G2, V> AnyVisitor for FirstTickerVisitor<'a, G1, G2, V>
    where
        G1: Group,
        G2: Group,
        V: AnyVisitorPair<VisitedG1 = G1, VisitedG2 = G2>,
        Error: Into<V::Error>,
    {
        type VisitedG = G1;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self) -> AnyVisitorResult<Self>
        where
            C1: Currency + MemberOf<G1>,
        {
            Tickers::<G2>::visit_any(
                self.ticker2,
                SecondTickerVisitor {
                    currency1: PhantomData::<C1>,
                    group2: PhantomData::<G2>,
                    visitor: self.visitor,
                },
            )
        }
    }

    struct SecondTickerVisitor<C1, G2, V>
    where
        C1: Currency,
        V: AnyVisitorPair,
    {
        currency1: PhantomData<C1>,
        group2: PhantomData<G2>,
        visitor: V,
    }
    impl<C1, G2, V> AnyVisitor for SecondTickerVisitor<C1, G2, V>
    where
        C1: Currency + MemberOf<V::VisitedG1>,
        G2: Group,
        V: AnyVisitorPair<VisitedG2 = G2>,
    {
        type VisitedG = G2;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self) -> AnyVisitorResult<Self>
        where
            C2: Currency + MemberOf<Self::VisitedG>,
        {
            self.visitor.on::<C1, C2>()
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol_any::GroupVisit,
        test::{
            Expect, ExpectPair, ExpectUnknownCurrency, SubGroup, SubGroupTestC1, SuperGroup,
            SuperGroupTestC1, SuperGroupTestC2,
        },
        Currency, Group, MemberOf, Tickers,
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<SuperGroupTestC1, _>::default();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(SuperGroupTestC1::TICKER, v_usdc)
        );

        let v_nls = Expect::<SuperGroupTestC2, _>::default();
        assert_eq!(
            Ok(true),
            Tickers::<<SuperGroupTestC2 as Currency>::Group>::visit_any(
                SuperGroupTestC2::TICKER,
                v_nls
            )
        );

        assert_eq!(
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SubGroup>,
                SuperGroup,
            >(SubGroupTestC1::BANK_SYMBOL)),
            Tickers::<SuperGroup>::visit_any(
                SubGroupTestC1::BANK_SYMBOL,
                ExpectUnknownCurrency::<SuperGroup>::new()
            )
        );
    }

    #[test]
    fn visit_any_not_in_group() {
        let v_usdc = Expect::<SuperGroupTestC1, _>::default();
        assert_eq!(
            Ok(false),
            Tickers::<SuperGroup>::visit_any(SubGroupTestC1::TICKER, v_usdc)
        );

        let v_usdc = ExpectUnknownCurrency::<SubGroup>::new();
        assert_eq!(
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SuperGroup>,
                SubGroup,
            >(SuperGroupTestC1::TICKER)),
            Tickers::<SubGroup>::visit_any(SuperGroupTestC1::TICKER, v_usdc)
        );
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            Tickers::visit_any(DENOM, ExpectUnknownCurrency::<SuperGroup>::new()),
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SubGroup>,
                SuperGroup,
            >(DENOM)),
        );
    }

    #[test]
    fn visit_any_tickers() {
        visit_any_tickers_ok::<SuperGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_tickers_ok::<SuperGroup, SuperGroup, SuperGroupTestC2, SuperGroupTestC2>();
        visit_any_tickers_ok::<SubGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC1>();
        visit_any_tickers_ok::<SuperGroup, SubGroup, SuperGroupTestC2, SubGroupTestC1>();

        visit_any_tickers_ok::<SuperGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC2>();
        visit_any_tickers_ok::<SuperGroup, SuperGroup, SubGroupTestC1, SubGroupTestC1>();
        visit_any_tickers_fail::<SubGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
    }

    fn visit_any_tickers_ok<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: Currency + MemberOf<G1>,
        C2: Currency + MemberOf<G2>,
    {
        let v_c1_c2 = ExpectPair::<C1, G1, C2, G2>::default();
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
        let v_c1_c2 = ExpectPair::<C1, G1, C2, G2>::default();
        assert!(super::visit_any_on_tickers::<G1, G2, _>(C1::TICKER, C2::TICKER, v_c1_c2).is_err());
    }
}
