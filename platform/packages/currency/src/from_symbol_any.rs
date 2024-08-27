use crate::{
    error::Error, group::MemberOf, matcher, Currency, CurrencyDTO, CurrencyDef,
    MaybeAnyVisitResult, Symbol, SymbolSlice,
};

use super::Group;

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

    // TODO suppose passing def as CurrencyDTO<G>, where G:Group, C: Currency + MembedOf<G> would help the compiler to eliminate a bunch of monomorphized functions
    fn on<C>(self, def: &C) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<VisitedG> + MemberOf<Self::VisitorG>;
}
pub trait AnyVisitorPair {
    type VisitedG: Group;

    type Output;
    type Error;

    fn on<C1, C2>(
        self,
        dto1: &CurrencyDTO<Self::VisitedG>,
        dto2: &CurrencyDTO<Self::VisitedG>,
    ) -> AnyVisitorPairResult<Self>
    where
        C1: Currency + MemberOf<Self::VisitedG>,
        C2: Currency + MemberOf<Self::VisitedG>;
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

pub fn visit_any_on_currencies<G, V>(
    currency1: CurrencyDTO<G>,
    currency2: CurrencyDTO<G>,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitorPair<VisitedG = G>,
{
    currency1.into_currency_type(FirstTickerVisitor::<G, _>::new(currency2, visitor))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use crate::{Currency, CurrencyDTO, CurrencyDef, Group, MemberOf};

    use super::{AnyVisitor, AnyVisitorPair, AnyVisitorResult};

    pub struct FirstTickerVisitor<G, V>
    where
        G: Group,
        V: AnyVisitorPair,
    {
        currency2: CurrencyDTO<G>,
        visitor: V,
    }
    impl<G, V> FirstTickerVisitor<G, V>
    where
        G: Group,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: CurrencyDTO<G>, visitor: V) -> Self {
            Self {
                currency2: ticker2,
                visitor,
            }
        }
    }
    impl<G, V> AnyVisitor<G> for FirstTickerVisitor<G, V>
    where
        G: Group,
        V: AnyVisitorPair<VisitedG = G>,
    {
        type VisitorG = G;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self, def1: &C1) -> AnyVisitorResult<G, Self>
        where
            C1: CurrencyDef,
            C1::Group: MemberOf<G>,
        {
            self.currency2.into_currency_type(SecondTickerVisitor {
                c: PhantomData::<C1>,
                def1: def1.dto().into_super_group::<V::VisitedG>(),
                group: PhantomData::<G>,
                visitor: self.visitor,
            })
        }
    }

    struct SecondTickerVisitor<C1, G, V>
    where
        C1: Currency,
        V: AnyVisitorPair,
    {
        c: PhantomData<C1>,
        def1: CurrencyDTO<V::VisitedG>,
        group: PhantomData<G>,
        visitor: V,
    }
    impl<C1, G, V> AnyVisitor<G> for SecondTickerVisitor<C1, G, V>
    where
        C1: Currency + MemberOf<G> + MemberOf<V::VisitedG>,
        G: Group,
        V: AnyVisitorPair<VisitedG = G>,
    {
        type VisitorG = G;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self, def2: &C2) -> AnyVisitorResult<G, Self>
        where
            C2: CurrencyDef + MemberOf<Self::VisitorG>,
            C2::Group: MemberOf<Self::VisitorG>,
        {
            self.visitor
                .on::<C1, C2>(&self.def1, &def2.dto().into_super_group::<V::VisitedG>())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol_any::GroupVisit,
        test::{
            Expect, ExpectPair, ExpectUnknownCurrency, SubGroup, SubGroupTestC10, SubGroupTestC6,
            SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
        },
        CurrencyDef, Group, MemberOf, Tickers,
    };

    #[test]
    fn visit_any() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(SuperGroupTestC1::ticker(), v_usdc.clone())
        );
        assert_eq!(
            Ok(Ok(true)),
            Tickers::<SuperGroup>::maybe_visit_any(SuperGroupTestC1::ticker(), v_usdc)
        );

        let v_nls = Expect::<SuperGroupTestC2, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(SuperGroupTestC2::ticker(), v_nls)
        );

        assert_eq!(
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SubGroup>,
                SuperGroup,
            >(SubGroupTestC10::bank())),
            Tickers::<SuperGroup>::visit_any(
                SubGroupTestC10::bank(),
                ExpectUnknownCurrency::<SuperGroup>::new()
            )
        );
        let v = ExpectUnknownCurrency::<SuperGroup>::new();
        assert_eq!(
            Err(v.clone()),
            Tickers::<SuperGroup>::maybe_visit_any(SubGroupTestC10::bank(), v)
        );
    }

    #[test]
    fn visit_super_group() {
        assert_eq!(
            Ok(true),
            Tickers::<SuperGroup>::visit_any(
                SubGroupTestC10::ticker(),
                Expect::<SubGroupTestC10, SuperGroup, SuperGroup>::new()
            )
        );

        assert_eq!(
            Ok(true),
            Tickers::<SubGroup>::visit_member_any(
                SubGroupTestC10::ticker(),
                Expect::<SubGroupTestC10, SubGroup, SuperGroup>::new()
            )
        );
    }

    #[test]
    fn visit_any_not_in_group() {
        let v_usdc = Expect::<SuperGroupTestC1, SuperGroup, SuperGroup>::new();
        assert_eq!(
            Ok(false),
            Tickers::<SuperGroup>::visit_any(SubGroupTestC10::ticker(), v_usdc)
        );

        let v_usdc = ExpectUnknownCurrency::<SubGroup>::new();
        assert_eq!(
            Err(Error::not_in_currency_group::<
                _,
                Tickers::<SuperGroup>,
                SubGroup,
            >(SuperGroupTestC1::ticker())),
            Tickers::<SubGroup>::visit_any(SuperGroupTestC1::ticker(), v_usdc)
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
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC6>();
        visit_any_currencies_ok::<SubGroup, SubGroupTestC10, SubGroupTestC6>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC10>();

        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SubGroupTestC10>();
    }

    fn visit_any_currencies_ok<VisitedG, CDef1, CDef2>()
    where
        VisitedG: Group,
        VisitedG: Group,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<VisitedG>,
    {
        let dto1 = crate::dto::<CDef1, _>();
        let dto2 = crate::dto::<CDef2, _>();
        let v_c1_c2 = ExpectPair::<VisitedG, VisitedG, _, _>::new(&dto1, &dto2);
        assert_eq!(
            Ok(true),
            super::visit_any_on_currencies::<VisitedG, _>(
                crate::dto::<CDef1, VisitedG>(),
                crate::dto::<CDef2, VisitedG>(),
                v_c1_c2
            )
        );
    }
}
