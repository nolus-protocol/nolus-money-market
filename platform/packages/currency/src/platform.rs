use std::{borrow::Borrow, marker::PhantomData};

use serde::{Deserialize, Serialize};

use crate::{
    AnyVisitor, CurrencyDTO, CurrencyDef, Definition, Group, Matcher, MaybeAnyVisitResult,
    MaybePairsVisitorResult, MemberOf, PairsGroup, PairsVisitor, group::FilterMapT,
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
        PlatformCurrencies::with_filter(f)
    }

    // fn find_map<FindMap>(_f: FindMap) -> Result<FindMap::Outcome, FindMap>
    // where
    //     FindMap: FindMapT<Self>,
    // {
    //     todo!()
    // }

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

/// Iterator over platform currency types mapped to some values
struct PlatformCurrencies<FilterMap, FilterMapRef> {
    f: FilterMapRef,
    _f_type: PhantomData<FilterMap>,
    next: Option<Item>,
}

// trait Item {
//     fn next(self) -> Option<impl Item>;
// }

// 1. specialization on the Currency type instead to using TypeId. It has to be `Iterator`,
// so we need a smaller abstraction `NextType` that calls the filter, then if in iteration
// -- [pros] immutable

struct CurrencyItem<Currency>(PhantomData<Currency>);

impl<C> Default for CurrencyItem<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<C> CurrencyItem<C>
where
    C: CurrencyDef,
{
    // the &self argument is not necessary for the implementation but is needed for easier message passing
    fn filter_map<VisitedG, FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        VisitedG: Group,
        C: PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
        FilterMap: FilterMapT<VisitedG>,
    {
        filter_map.on::<C>(C::dto())
    }

    // fn find_map(find_map: FindMap) -> Result<FindMap::Outcome, FindMap> {
    //     find_map.on::<Currency>(Currency::dto())
    // }
}

// ======== START GENERATED CODE =========
// An alternative to TypeId
enum Item {
    Nls(CurrencyItem<Nls>),
    Stable(CurrencyItem<Stable>),
}

impl Item {
    fn first() -> Option<Item> {
        Some(Self::Nls(CurrencyItem::default()))
    }

    fn next(&self) -> Option<Item> {
        match self {
            Item::Nls(_item) => Some(Self::Stable(CurrencyItem::default())),
            Item::Stable(_item) => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        // C: PairsGroup<CommonGroup = VisitedG::TopG>,
        // C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
        FilterMap: FilterMapT<PlatformGroup>,
    {
        match *self {
            Item::Nls(ref item) => item.filter_map(filter_map),
            Item::Stable(ref item) => item.filter_map(filter_map),
        }
    }
}

// impl CurrencyItem<Nls> {
//     fn next(self) -> Option<CurrencyItem<Stable>> {
//         Some(CurrencyItem::<Stable>())
//     }
// }

// impl CurrencyItem<Stable> {
//     fn next(self) -> Option<CurrencyItem<Stable>> {
//         None
//     }
// }
// ======== END GENERATED CODE =========

impl<FilterMap, FilterMapRef> PlatformCurrencies<FilterMap, FilterMapRef>
where
    FilterMap: FilterMapT<PlatformGroup>,
    FilterMapRef: Borrow<FilterMap>,
{
    fn with_filter(f: FilterMapRef) -> Self {
        Self {
            f,
            _f_type: PhantomData,
            next: Item::first(),
        }
    }
}

impl<FilterMap, FilterMapRef> Iterator for PlatformCurrencies<FilterMap, FilterMapRef>
where
    FilterMap: FilterMapT<PlatformGroup>,
    FilterMapRef: Borrow<FilterMap>,
{
    type Item = FilterMap::Outcome;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        while result.is_none() {
            match self.next {
                Some(ref current) => {
                    result = current.filter_map(self.f.borrow());
                    self.next = current.next();
                }
                None => {
                    break;
                }
            }
            // result = self.next_map();
        }
        result
    }
}

#[cfg(test)]
mod test {

    use crate::{
        CurrencyDef,
        platform::{Nls, Stable},
        test::{
            SubGroupTestC6,
            filter::{Dto, FindByTicker},
        },
    };

    use super::PlatformCurrencies;

    #[test]
    fn enumerate_all() {
        let filter = Dto::default();
        let mut iter = PlatformCurrencies::with_filter(filter);
        assert_eq!(Some(Nls::dto()), iter.next().as_ref());
        assert_eq!(Some(Stable::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SubGroupTestC6::ticker(), Stable::ticker());
        let mut iter = PlatformCurrencies::with_filter(filter);
        assert_eq!(Some(Stable::dto()), iter.next().as_ref());
        assert_eq!(None, iter.next().as_ref());
    }
}
