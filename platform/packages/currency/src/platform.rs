use std::{any::TypeId, borrow::Borrow, marker::PhantomData};

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
    next: Option<TypeId>,
}

impl<FilterMap, FilterMapRef> PlatformCurrencies<FilterMap, FilterMapRef>
where
    FilterMap: FilterMapT<PlatformGroup>,
    FilterMapRef: Borrow<FilterMap>,
{
    fn with_filter(f: FilterMapRef) -> Self {
        Self {
            f,
            _f_type: PhantomData,
            next: Some(TypeId::of::<Nls>()),
        }
    }

    fn next_map(&mut self) -> Option<FilterMap::Outcome> {
        debug_assert!(self.next.is_some());

        // TODO define `const` for each of the currencies
        // once `const fn TypeId::of` gets stabilized
        // and switch from `if-else` to `match`
        let nls_type = TypeId::of::<Nls>();
        let stable_type = TypeId::of::<Stable>();

        self.next.and_then(|next_type| {
            let filter = self.f.borrow();
            if next_type == nls_type {
                self.next = Some(stable_type);
                filter.on::<Nls>(Nls::dto())
            } else if next_type == stable_type {
                self.next = None;
                filter.on::<Stable>(Stable::dto())
            } else {
                unimplemented!("Unknown type found!")
            }
        })
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
        while result.is_none() && self.next.is_some() {
            result = self.next_map();
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
