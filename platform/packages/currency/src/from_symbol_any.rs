use crate::{
    error::Error, group::MemberOf, matcher, pairs::PairsGroup, Currency, CurrencyDTO, CurrencyDef,
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
    VisitedG: Group,
{
    type VisitorG: Group;

    type Output;
    type Error;

    // TODO suppose passing def as CurrencyDTO<G>, where G:Group, C: Currency + MembedOf<G> would help the compiler to eliminate a bunch of monomorphized functions
    fn on<C>(self, def: &C) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<VisitedG> + MemberOf<Self::VisitorG>; // cannot deduce the same bounds for C, as MemberOf defines a sub-group where Self belongs to
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

pub trait GroupVisit
where
    Self: Symbol,
{
    fn visit_any<V>(symbol: &SymbolSlice, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self::Group, VisitorG = Self::Group>,
        Self::Group: MemberOf<Self::Group>,
        Error: Into<V::Error>,
    {
        Self::maybe_visit_any(symbol, visitor).unwrap_or_else(|_| {
            Err(Error::not_in_currency_group::<_, Self, Self::Group>(symbol).into())
        })
    }

    fn maybe_visit_any<V>(symbol: &SymbolSlice, visitor: V) -> MaybeAnyVisitResult<Self::Group, V>
    where
        V: AnyVisitor<Self::Group, VisitorG = Self::Group>,
        Self::Group: MemberOf<Self::Group>,
    {
        let matcher = matcher::symbol_matcher::<Self>(symbol);
        <Self::Group as Group>::maybe_visit(&matcher, visitor)
    }
}
impl<T> GroupVisit for T
where
    T: Symbol,
    T::Group: MemberOf<T::Group>,
{
}

pub fn visit_any_on_currencies<G, V>(
    currency1: CurrencyDTO<G>,
    currency2: CurrencyDTO<G>,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G: Group + PairsGroup<CommonGroup = G>,
    V: AnyVisitorPair<VisitedG = G>,
    Error: Into<V::Error>,
{
    currency1.into_pair_member_type::<G, _>(FirstTickerVisitor::new(currency2, visitor))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use crate::{
        error::Error,
        pairs::{PairsGroup, PairsVisitor, PairsVisitorResult},
        Currency, CurrencyDTO, CurrencyDef, Group, MemberOf,
    };

    use super::AnyVisitorPair;

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
        G: Group + PairsGroup<CommonGroup = G>,
        V: AnyVisitorPair,
    {
        pub fn new(ticker2: CurrencyDTO<G>, visitor: V) -> Self {
            Self {
                currency2: ticker2,
                visitor,
            }
        }
    }
    impl<G, V> PairsVisitor for FirstTickerVisitor<G, V>
    where
        G: Group + PairsGroup<CommonGroup = G>,
        V: AnyVisitorPair<VisitedG = G>,
        Error: Into<V::Error>,
    {
        type VisitedG = G::CommonGroup;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C1>(self, def: &CurrencyDTO<C1::Group>) -> PairsVisitorResult<Self>
        where
            C1: CurrencyDef + PairsGroup<CommonGroup = Self::VisitedG>,
            C1::Group: Group + MemberOf<Self::VisitedG>,
        {
            let def1 = def.into_super_group();
            self.currency2
                .may_into_pair_member_type::<C1, _>(SecondTickerVisitor {
                    c: PhantomData::<C1>,
                    def1,
                    group: PhantomData::<G>,
                    visitor: self.visitor,
                })
                .unwrap_or_else(|_| Err(Error::not_in_pool_with(&def1, &self.currency2).into()))
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
    impl<C1, G, V> PairsVisitor for SecondTickerVisitor<C1, G, V>
    where
        C1: Currency + MemberOf<G> + MemberOf<V::VisitedG>,
        G: Group,
        V: AnyVisitorPair<VisitedG = G>,
    {
        type VisitedG = G;

        type Output = <V as AnyVisitorPair>::Output;
        type Error = <V as AnyVisitorPair>::Error;

        fn on<C2>(self, def: &CurrencyDTO<C2::Group>) -> PairsVisitorResult<Self>
        where
            C2: CurrencyDef,
            C2::Group: Group + MemberOf<Self::VisitedG>,
        {
            self.visitor
                .on::<C1, C2>(&self.def1, &def.into_super_group())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        error::Error,
        from_symbol_any::GroupVisit,
        pairs::PairsGroup,
        test::{
            Expect, ExpectPair, ExpectUnknownCurrency, SubGroup, SubGroupTestC10, SubGroupTestC6,
            SuperGroup, SuperGroupTestC1, SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC5,
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
        //Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC6>();

        // visit_any_currencies_ok::<SubGroup, SubGroupTestC10, SubGroupTestC6>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SubGroupTestC6>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC2>();

        // visit_any_currencies_nok::<SubGroup, SubGroupTestC10, SubGroupTestC10>();
        visit_any_currencies_nok::<SuperGroup, SubGroupTestC10, SubGroupTestC10>();

        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC3>();
        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC3, SuperGroupTestC1>();

        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC5>();
        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC5, SuperGroupTestC1>();
    }

    fn visit_any_currencies_ok<VisitedG, CDef1, CDef2>()
    where
        VisitedG: Group + PairsGroup<CommonGroup = VisitedG>,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<CDef1::Group> + MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<CDef2::Group> + MemberOf<VisitedG>,
    {
        assert_eq!(
            Ok(true),
            visit_any_currencies_int::<VisitedG, CDef1, CDef2>()
        );
    }

    fn visit_any_currencies_nok<VisitedG, CDef1, CDef2>()
    where
        VisitedG: Group + PairsGroup<CommonGroup = VisitedG>,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<CDef1::Group> + MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<CDef2::Group> + MemberOf<VisitedG>,
    {
        assert_eq!(
            Err(Error::not_in_pool_with(
                &CDef1::definition().dto().into_super_group::<VisitedG>(),
                &CDef2::definition().dto().into_super_group::<VisitedG>()
            )),
            visit_any_currencies_int::<VisitedG, CDef1, CDef2>()
        );
    }

    fn visit_any_currencies_int<VisitedG, CDef1, CDef2>() -> Result<bool, Error>
    where
        VisitedG: Group + PairsGroup<CommonGroup = VisitedG>,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<CDef1::Group> + MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<CDef2::Group> + MemberOf<VisitedG>,
    {
        let dto1 = crate::dto::<CDef1, _>();
        let dto2 = crate::dto::<CDef2, _>();
        let v_c1_c2 = ExpectPair::<VisitedG, _, _>::new(&dto1, &dto2);
        super::visit_any_on_currencies::<VisitedG, _>(dto1, dto2, v_c1_c2)
    }
}
