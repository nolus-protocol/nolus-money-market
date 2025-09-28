use std::marker::PhantomData;

use crate::{
    CurrencyDTO, CurrencyDef, Group, GroupFindMapT, Matcher, MemberOf, PairsFindMapT, PairsGroup,
    PairsVisitor, error::Error,
};

use self::impl_any_tickers::FirstTickerVisitor;

pub trait AnyVisitor<VisitedG>
where
    VisitedG: Group,
{
    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Self::Outcome
    where
        // cannot simplify to `C: Currency + MemberOf<VisitedG> + MemberOf<VisitedG::TopG> + ...`
        // due to the lack of relation to the type argument of the `CurrencyDTO` argument
        C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>;
}

pub trait InPoolWith<C> {}

pub trait AnyVisitorPair {
    type VisitedG: Group<TopG = Self::VisitedG>;

    type Outcome;

    fn on<C1, C2>(
        self,
        dto1: &CurrencyDTO<Self::VisitedG>,
        dto2: &CurrencyDTO<Self::VisitedG>,
    ) -> Self::Outcome
    where
        C1: CurrencyDef,
        C1::Group: MemberOf<Self::VisitedG>,
        C2: CurrencyDef + InPoolWith<C1>,
        C2::Group: MemberOf<Self::VisitedG>;
}

pub(crate) struct MatchThenVisit<M, V, VisitedG> {
    matcher: M,
    v: V,
    _visited_g: PhantomData<VisitedG>,
}

impl<M, V, VisitedG> MatchThenVisit<M, V, VisitedG> {
    pub fn new(matcher: M, v: V) -> Self {
        Self {
            matcher,
            v,
            _visited_g: PhantomData,
        }
    }

    pub fn release_visitor(self) -> V {
        self.v
    }
}

impl<M, V, VisitedG> GroupFindMapT for MatchThenVisit<M, V, VisitedG>
where
    M: Matcher,
    V: AnyVisitor<VisitedG>,
    VisitedG: Group,
{
    type TargetG = VisitedG;
    type Outcome = V::Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
    {
        if self.matcher.r#match(def.definition()) {
            Ok(self.v.on::<C>(def))
        } else {
            Err(self)
        }
    }
}

impl<M, V, VisitedG> PairsFindMapT for MatchThenVisit<M, V, VisitedG>
where
    M: Matcher,
    V: PairsVisitor,
{
    type Pivot = V::Pivot;
    type Outcome = V::Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef
            + InPoolWith<Self::Pivot>
            + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
        C::Group: MemberOf<<Self::Pivot as PairsGroup>::CommonGroup>,
    {
        if self.matcher.r#match(def.definition()) {
            Ok(self.v.on::<C>(def))
        } else {
            Err(self)
        }
    }
}

/// Resolve a pair of currencies and execute the visitor
///
/// Return an [Error::NotInPoolWith] if the provided currencies are an unknown pair, otherwise the visiting result
pub fn visit_any_on_currencies<V>(
    currency1: CurrencyDTO<V::VisitedG>,
    currency2: CurrencyDTO<V::VisitedG>,
    visitor: V,
) -> Result<V::Outcome, Error>
where
    V: AnyVisitorPair,
{
    currency1.into_currency_type(FirstTickerVisitor::new(currency1, currency2, visitor))
}

mod impl_any_tickers {
    use std::marker::PhantomData;

    use crate::{
        Currency, CurrencyDTO, CurrencyDef, Group, MemberOf,
        error::Error,
        pairs::{PairsGroup, PairsVisitor},
        visit_any::InPoolWith,
    };

    use super::{AnyVisitor, AnyVisitorPair};

    pub struct FirstTickerVisitor<V>
    where
        V: AnyVisitorPair,
    {
        currency1: CurrencyDTO<V::VisitedG>,
        currency2: CurrencyDTO<V::VisitedG>,
        visitor: V,
    }
    impl<V> FirstTickerVisitor<V>
    where
        V: AnyVisitorPair,
    {
        pub fn new(
            currency1: CurrencyDTO<V::VisitedG>,
            currency2: CurrencyDTO<V::VisitedG>,
            visitor: V,
        ) -> Self {
            Self {
                currency1,
                currency2,
                visitor,
            }
        }
    }
    impl<V> AnyVisitor<V::VisitedG> for FirstTickerVisitor<V>
    where
        V: AnyVisitorPair,
    {
        type Outcome = Result<<V as AnyVisitorPair>::Outcome, Error>;

        fn on<C1>(self, def: &CurrencyDTO<C1::Group>) -> Self::Outcome
        where
            C1: CurrencyDef + PairsGroup<CommonGroup = <V::VisitedG as Group>::TopG>,
            C1::Group: MemberOf<V::VisitedG> + MemberOf<<V::VisitedG as Group>::TopG>, // TODO since V::VisitedG === Self::VisitorG, do we need them both?
        {
            debug_assert_eq!(def, &self.currency1);
            self.currency2
                .may_into_pair_member_type({
                    let Self {
                        currency1,
                        currency2,
                        visitor,
                    } = self;

                    SecondTickerVisitor {
                        c: PhantomData::<C1>,
                        currency1,
                        currency2,
                        visitor,
                    }
                })
                .map_err(|_| Error::not_in_pool_with(&self.currency1, &self.currency2))
        }
    }

    struct SecondTickerVisitor<C1, V>
    where
        C1: Currency,
        V: AnyVisitorPair,
    {
        c: PhantomData<C1>,
        currency1: CurrencyDTO<V::VisitedG>,
        currency2: CurrencyDTO<V::VisitedG>,
        visitor: V,
    }
    impl<C1, V> PairsVisitor for SecondTickerVisitor<C1, V>
    where
        C1: CurrencyDef + PairsGroup,
        C1::Group: MemberOf<V::VisitedG> + MemberOf<<C1 as PairsGroup>::CommonGroup>,
        V: AnyVisitorPair<VisitedG = C1::CommonGroup>,
    {
        type Pivot = C1;

        type Outcome = <V as AnyVisitorPair>::Outcome;

        fn on<C2>(self, currency: &CurrencyDTO<C2::Group>) -> Self::Outcome
        where
            C2: CurrencyDef + InPoolWith<Self::Pivot>,
            C2::Group: MemberOf<<Self::Pivot as PairsGroup>::CommonGroup>,
        {
            debug_assert_eq!(currency, &self.currency2);
            self.visitor.on::<C1, C2>(&self.currency1, &self.currency2)
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        CurrencyDef, Group, MemberOf,
        error::Error,
        test::{
            ExpectPair, SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1,
            SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
        },
    };

    #[test]
    fn visit_any_currencies() {
        //Pool pairs: 1:2, 1:4, 1:10, 2:3, 2:6, 2:10, 4:5, 5:10, 6:10
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC2>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SuperGroupTestC1>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC4>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC4, SuperGroupTestC1>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC1, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC1>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SuperGroupTestC3>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC3, SuperGroupTestC2>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC6>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC6, SuperGroupTestC2>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC2>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC4, SuperGroupTestC5>();
        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC5, SuperGroupTestC4>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC5, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC5>();

        // visit_any_currencies_ok::<SubGroup, SubGroupTestC10, SubGroupTestC6>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC6, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SubGroupTestC6>();

        visit_any_currencies_ok::<SuperGroup, SuperGroupTestC2, SubGroupTestC10>();
        visit_any_currencies_ok::<SuperGroup, SubGroupTestC10, SuperGroupTestC2>();

        visit_any_currencies_nok::<SuperGroup, SubGroupTestC10, SubGroupTestC10>();

        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC1, SuperGroupTestC3>();
        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC3, SuperGroupTestC1>();

        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC2, SuperGroupTestC4>();
        visit_any_currencies_nok::<SuperGroup, SuperGroupTestC4, SuperGroupTestC2>();
    }

    #[track_caller]
    fn visit_any_currencies_ok<VisitedG, CDef1, CDef2>()
    where
        VisitedG: Group<TopG = VisitedG>,
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

    #[track_caller]
    fn visit_any_currencies_nok<VisitedG, CDef1, CDef2>()
    where
        VisitedG: Group<TopG = VisitedG>,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<CDef1::Group> + MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<CDef2::Group> + MemberOf<VisitedG>,
    {
        assert_eq!(
            Err(Error::not_in_pool_with(
                &CDef1::dto().into_super_group::<VisitedG>(),
                &CDef2::dto().into_super_group::<VisitedG>()
            )),
            visit_any_currencies_int::<VisitedG, CDef1, CDef2>()
        );
    }

    fn visit_any_currencies_int<VisitedG, CDef1, CDef2>() -> Result<bool, Error>
    where
        VisitedG: Group<TopG = VisitedG>,
        CDef1: CurrencyDef,
        CDef1::Group: MemberOf<CDef1::Group> + MemberOf<VisitedG>,
        CDef2: CurrencyDef,
        CDef2::Group: MemberOf<CDef2::Group> + MemberOf<VisitedG>,
    {
        let dto1 = crate::dto::<CDef1, _>();
        let dto2 = crate::dto::<CDef2, _>();
        let v_c1_c2 = ExpectPair::<VisitedG, _, _>::new(&dto1, &dto2);
        super::visit_any_on_currencies(dto1, dto2, v_c1_c2)
    }
}
