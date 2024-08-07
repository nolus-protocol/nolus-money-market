use crate::{
    error::Error, group::MemberOf, matcher, CurrencyDTO, Definition, MaybeAnyVisitResult, Symbol,
    SymbolSlice,
};

use super::{Currency, Group};

use self::impl_any_tickers::FirstTickerVisitor;

pub type AnyVisitorResult<VisitedG, Visitor> =
    Result<<Visitor as AnyVisitor<VisitedG>>::Output, <Visitor as AnyVisitor<VisitedG>>::Error>;
pub type AnyVisitorPairResult<V> =
    Result<<V as AnyVisitorPair>::Output, <V as AnyVisitorPair>::Error>;

pub trait AnyVisitor<VisitedG>
where
    VisitedG: Group + MemberOf<Self::VisitorG>,
{
    type VisitorG: Group;

    type Output;
    type Error;

    fn on<C>(self) -> AnyVisitorResult<VisitedG, Self>
    where
        C: Currency + MemberOf<VisitedG> + MemberOf<Self::VisitorG> + Definition;
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
        V: AnyVisitor<Self::Group, VisitorG = Self::Group>,
        Error: Into<V::Error>,
    {
        Self::maybe_visit_any(symbol, visitor).unwrap_or_else(|_| {
            Err(Error::not_in_currency_group::<_, Self, Self::Group>(symbol).into())
        })
    }

    fn maybe_visit_any<V>(symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<Self::Group, V>
    where
        V: AnyVisitor<Self::Group, VisitorG = Self::Group>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        Self::Group::maybe_visit(&matcher, visitor)
    }

    fn visit_member_any<V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self::Group>,
        Self::Group: MemberOf<V::VisitorG>,
        Error: Into<V::Error>,
    {
        Self::maybe_visit_member_any(symbol, visitor).unwrap_or_else(|_| {
            Err(Error::not_in_currency_group::<_, Self, Self::Group>(symbol).into())
        })
    }

    fn maybe_visit_member_any<V>(
        symbol: &SymbolSlice,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self::Group, V>
    where
        V: AnyVisitor<Self::Group>,
        Self::Group: MemberOf<V::VisitorG>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        Self::Group::maybe_visit_super_visitor(&matcher, visitor)
    }
}
impl<T> GroupVisit for T where T: Symbol {}

pub fn visit_any_on_currencies<G1, G2, V>(
    currency1: CurrencyDTO<G1>,
    currency2: CurrencyDTO<G2>,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G1: Group,
    G2: Group,
    V: AnyVisitorPair<VisitedG1 = G1, VisitedG2 = G2>,
{
    currency1.into_currency_type(FirstTickerVisitor::<G1, G2, _>::new(currency2, visitor))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use crate::{Currency, CurrencyDTO, Group, MemberOf};

    use super::{AnyVisitor, AnyVisitorPair, AnyVisitorResult};

    pub struct FirstTickerVisitor<G1, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        group1: PhantomData<G1>,
        currency2: CurrencyDTO<G2>,
        visitor: V,
    }
    impl<G1, G2, V> FirstTickerVisitor<G1, G2, V>
    where
        G2: Group,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: CurrencyDTO<G2>, visitor: V) -> Self {
            Self {
                group1: PhantomData::<G1>,
                currency2: ticker2,
                visitor,
            }
        }
    }
    impl<G1, G2, V> AnyVisitor<G1> for FirstTickerVisitor<G1, G2, V>
    where
        G1: Group,
        G2: Group,
        V: AnyVisitorPair<VisitedG1 = G1, VisitedG2 = G2>,
    {
        type VisitorG = G1;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self) -> AnyVisitorResult<G1, Self>
        where
            C1: Currency + MemberOf<G1>,
        {
            self.currency2.into_currency_type(SecondTickerVisitor {
                currency1: PhantomData::<C1>,
                group2: PhantomData::<G2>,
                visitor: self.visitor,
            })
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
    impl<C1, G2, V> AnyVisitor<G2> for SecondTickerVisitor<C1, G2, V>
    where
        C1: Currency + MemberOf<V::VisitedG1>,
        G2: Group,
        V: AnyVisitorPair<VisitedG2 = G2>,
    {
        type VisitorG = G2;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self) -> AnyVisitorResult<G2, Self>
        where
            C2: Currency + MemberOf<Self::VisitorG>,
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
        Currency, CurrencyDTO, Definition, Group, MemberOf, Tickers,
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::default();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(SuperGroupTestC1::TICKER, v_usdc.clone())
        );
        assert_eq!(
            Ok(Ok(true)),
            Tickers::<SuperGroup>::maybe_visit_any(SuperGroupTestC1::TICKER, v_usdc)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::default();
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
        let v = ExpectUnknownCurrency::<SuperGroup>::new();
        assert_eq!(
            Err(v.clone()),
            Tickers::<SuperGroup>::maybe_visit_any(SubGroupTestC1::BANK_SYMBOL, v)
        );
    }

    #[test]
    fn visit_super_group() {
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(
                SubGroupTestC1::TICKER,
                Expect::<SubGroupTestC1, SuperGroup, SuperGroup>::default()
            )
        );

        assert_eq!(
            Ok(true),
            Tickers::<SubGroup>::visit_member_any(
                SubGroupTestC1::TICKER,
                Expect::<SubGroupTestC1, SubGroup, SuperGroup>::default()
            )
        );
    }

    #[test]
    fn visit_any_not_in_group() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::default();
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
            Tickers::<SuperGroup>::visit_any(DENOM, ExpectUnknownCurrency::<SuperGroup>::new()),
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SubGroup>,
                SuperGroup,
            >(DENOM)),
        );
    }

    #[test]
    fn visit_any_currencies() {
        visit_any_currencies_ok::<SuperGroup, SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SuperGroup, SuperGroupTestC2, SuperGroupTestC2>();
        visit_any_currencies_ok::<SubGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC1>();
        visit_any_currencies_ok::<SuperGroup, SubGroup, SuperGroupTestC2, SubGroupTestC1>();

        visit_any_currencies_ok::<SuperGroup, SuperGroup, SubGroupTestC1, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SuperGroup, SubGroupTestC1, SubGroupTestC1>();
    }

    fn visit_any_currencies_ok<G1, G2, C1, C2>()
    where
        G1: Group,
        G2: Group,
        C1: Currency + MemberOf<G1>,
        C2: Currency + MemberOf<G2>,
    {
        let v_c1_c2 = ExpectPair::<C1, G1, C2, G2>::default();
        assert_eq!(
            Ok(true),
            super::visit_any_on_currencies::<G1, G2, _>(
                CurrencyDTO::<G1>::from_currency_type::<C1>(),
                CurrencyDTO::<G2>::from_currency_type::<C2>(),
                v_c1_c2
            )
        );
    }
}
