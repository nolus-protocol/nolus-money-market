use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::{
    CurrencyDTO, CurrencyDef, Definition, Group, GroupFilterMap, GroupFindMap, MemberOf,
    PairsFindMap, PairsGroup, group,
    pairs::{self, PairedWith, PairedWithList},
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

    type PairedWith = StablePairedWithList;

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMap<Pivot = Self>,
    {
        pairs::find(find_map)
    }
}

pub struct StablePairedWithList;

impl<Pivot> PairedWithList<Pivot> for StablePairedWithList
where
    Pivot: PairsGroup,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: pairs::Visitor<Pivot>,
    {
        unimplemented!("The 'Stable' platform currency used in pairs resolution!")
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

    type PairedWith = NlsPairedWithList;

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMap<Pivot = Self>,
    {
        pairs::find(find_map)
    }
}

pub struct NlsPairedWithList;

impl<Pivot> PairedWithList<Pivot> for NlsPairedWithList
where
    Pivot: PairsGroup,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: pairs::Visitor<Pivot>,
    {
        unimplemented!("The 'Nls' platform currency used in pairs resolution!")
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct PlatformGroup;
impl Group for PlatformGroup {
    const DESCR: &'static str = "platform currencies";

    type TopG = Self;

    type Members = (Nls, (Stable,));

    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        group::non_recursive_filter_map(filter_map)
    }

    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = Self>,
    {
        group::non_recursive_find_map(find_map)
    }
}

impl MemberOf<Self> for PlatformGroup {}

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
