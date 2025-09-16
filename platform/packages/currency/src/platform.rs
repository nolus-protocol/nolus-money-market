use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::{
    CurrencyDTO, CurrencyDef, Definition, Group, MemberOf, PairsGroup,
    group::{self, CurrenciesMapping, FilterMapT, FindMapT, GroupMember},
    pairs::FindMapT as PairsFindMapT,
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

    fn find_map<FindMap>(_f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
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

    fn find_map<FindMap>(_f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
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
        FilterMap: FilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap>,
    {
        CurrenciesMapping::<_, Item, _, _>::with_filter(f)
    }

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = Self>,
    {
        group::find_map::<_, Item, _>(f)
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
        FilterMap: FilterMapT<VisitedG = PlatformGroup>,
    {
        match *self {
            Item::Nls() => filter_map.on::<Nls>(Nls::dto()),
            Item::Stable() => filter_map.on::<Stable>(Stable::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = PlatformGroup>,
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

    use crate::{
        CurrencyDef, Group, Tickers,
        matcher::symbol_matcher,
        platform::{Nls, PlatformGroup, Stable},
        test::{
            FindCurrencyBySymbol, SubGroupTestC6, {Dto, FindByTicker},
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
        let matcher = symbol_matcher::<Tickers<PlatformGroup>>(Nls::ticker());
        assert_eq!(
            Nls::dto(),
            &PlatformGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).unwrap()
        );

        let matcher = symbol_matcher::<Tickers<PlatformGroup>>(Stable::ticker());
        assert_eq!(
            Stable::dto(),
            &PlatformGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).unwrap()
        );

        let matcher = symbol_matcher::<Tickers<PlatformGroup>>("unknown ticker");
        assert!(PlatformGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).is_err());
    }
}
