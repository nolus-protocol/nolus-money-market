use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::{
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor,
    group::{CurrenciesMapping, FilterMapT, FindMapT, GroupMember},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct Stable();

impl CurrencyDef for Stable {
    type Group = PlatformGroup;

    fn dto() -> &'static CurrencyDTO<Self::Group> {
        const { &CurrencyDTO::new(const { &Definition::new("STABLE", "N/A_N/A_N/A", "N/A_N/A_N/A", 0) }) }
    }
}
impl PairsGroup for Stable {
    type CommonGroup = PlatformGroup;

    fn maybe_visit<M, V>(_matcher: &M, _visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>,
    {
        unreachable!("The 'Stable' platform currency used in pairs resolution!")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
/// A 'platform'-only 'dex-independent' representation of Nls.
///
/// Intended to be used *only* until the TODO below gets done, and *only* in dex-independent usecases:
/// - LP rewards
/// - Relayers' tips
pub struct Nls(CurrencyDTO<PlatformGroup>);

impl CurrencyDef for Nls {
    type Group = PlatformGroup;

    fn dto() -> &'static CurrencyDTO<Self::Group> {
        &const {
            CurrencyDTO::new(
                const {
                    &Definition::new(
                        "NLS",
                        "unls",
                        // TODO Define trait PlatformCurrency as a super trait of Currency and
                        // merge NlsPlatform and Nls
                        "N/A_N/A_N/A",
                        6,
                    )
                },
            )
        }
    }
}

impl PairsGroup for Nls {
    type CommonGroup = PlatformGroup;

    fn maybe_visit<M, V>(_matcher: &M, _visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor,
    {
        unreachable!("The 'Nls' platform currency used in pairs resolution!")
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct PlatformGroup;
impl Group for PlatformGroup {
    const DESCR: &'static str = "platform currencies";
    type TopG = Self;

    fn filter_map<FilterMap, FilterMapRef>(
        f: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<Self>,
        FilterMapRef: Borrow<FilterMap>,
    {
        CurrenciesMapping::<_, Item, _, _>::with_filter(f)
    }

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<Self>,
    {
        let mut may_next = Item::first();
        let mut result = Err(f);
        while let Some(next) = may_next {
            match result {
                Ok(ref _result) => {
                    break;
                }
                Err(f) => {
                    result = next.find_map(f);
                }
            }
            may_next = next.next();
        }
        result
    }

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
    {
        Self::maybe_visit_member(matcher, visitor)
    }

    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>,
    {
        crate::maybe_visit_member::<_, Nls, Self::TopG, _>(matcher, visitor)
            .or_else(|v| MaybeAnyVisitResult::Ok(v.on::<Stable>(Stable::dto())))
        // we accept ANY currency to allow any stable@protocol to be a member
    }
}

impl MemberOf<Self> for PlatformGroup {}

// ======== START GENERATED CODE =========
enum Item {
    Nls(),
    Stable(),
}

impl GroupMember<PlatformGroup> for Item {
    fn first() -> Option<Self> {
        Some(Self::Nls())
    }

    fn next(&self) -> Option<Self> {
        match self {
            Item::Nls() => Some(Self::Stable()),
            Item::Stable() => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: FilterMapT<PlatformGroup>,
    {
        match *self {
            Item::Nls() => filter_map.on::<Nls>(Nls::dto()),
            Item::Stable() => filter_map.on::<Stable>(Stable::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<PlatformGroup>,
    {
        match *self {
            Item::Nls() => find_map.on::<Nls>(Nls::dto()),
            Item::Stable() => find_map.on::<Stable>(Stable::dto()),
        }
    }
}
// ======== END GENERATED CODE =========

#[cfg(test)]
mod test {

    use std::fmt::Debug;

    use crate::{
        CurrencyDTO, CurrencyDef, Group, Matcher, MemberOf, PairsGroup, Tickers,
        group::FindMapT,
        matcher::symbol_matcher,
        platform::{Nls, PlatformGroup, Stable},
        test::{
            SubGroupTestC6,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::default();
        //intentionally use the filter by ref to avoid its potential clone
        let mut iter = PlatformGroup::filter_map::<Dto<PlatformGroup>, _>(&filter);
        assert_eq!(Some(Nls::dto()), iter.next().as_ref());
        assert_eq!(Some(Stable::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SubGroupTestC6::ticker(), Stable::ticker());
        let mut iter = PlatformGroup::filter_map(filter);
        assert_eq!(Some(Stable::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn find() {
        struct FindCurrencyBySymbol<Matcher>(Matcher);

        impl<Matcher> Debug for FindCurrencyBySymbol<Matcher> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple("FindCurrencyBySymbol")
                    .field(&"matcher")
                    .finish()
            }
        }

        impl<MatcherImpl, VisitedG> FindMapT<VisitedG> for FindCurrencyBySymbol<MatcherImpl>
        where
            MatcherImpl: Matcher,
            VisitedG: Group,
        {
            type Outcome = CurrencyDTO<VisitedG>;

            fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
            where
                C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
                C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
            {
                if self.0.r#match(def.definition()) {
                    Ok(def.into_super_group())
                } else {
                    Err(self)
                }
            }
        }

        let matcher = symbol_matcher::<Tickers<PlatformGroup>>(Nls::ticker());
        assert_eq!(
            Nls::dto(),
            &PlatformGroup::find_map(FindCurrencyBySymbol(matcher)).unwrap()
        );

        let matcher = symbol_matcher::<Tickers<PlatformGroup>>(Stable::ticker());
        assert_eq!(
            Stable::dto(),
            &PlatformGroup::find_map(FindCurrencyBySymbol(matcher)).unwrap()
        );

        let matcher = symbol_matcher::<Tickers<PlatformGroup>>("unknown ticker");
        assert!(PlatformGroup::find_map(FindCurrencyBySymbol(matcher)).is_err());
    }
}
