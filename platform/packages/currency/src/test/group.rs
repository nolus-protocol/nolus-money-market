use std::borrow::Borrow;

use serde::Deserialize;

use crate::{
    CurrencyDTO, FindMapT, Group,
    group::{self, CurrenciesMapping, FilterMapT, MemberOf, SubFilterAdapter, SubGroupFindAdapter},
    pairs::{self, FindMapT as PairsFindMapT, PairsGroup},
    test::{
        sub::{Item as SubGroupItem, SubGroupTestC6Pairs, SubGroupTestC10Pairs},
        super_::{
            Item as SuperGroupItem, SuperGroupTestC1Pairs, SuperGroupTestC2Pairs,
            SuperGroupTestC3Pairs, SuperGroupTestC4Pairs, SuperGroupTestC5Pairs,
        },
    },
    visit_any::InPoolWith,
};

pub type SuperGroupTestC1 = impl_::TestC1;
pub type SuperGroupTestC2 = impl_::TestC2;
pub type SuperGroupTestC3 = impl_::TestC3;
pub type SuperGroupTestC4 = impl_::TestC4;
pub type SuperGroupTestC5 = impl_::TestC5;
pub type SubGroupTestC6 = impl_::TestC6;
pub type SubGroupTestC10 = impl_::TestC10;

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct SuperGroup {}

pub type SuperGroupCurrency = CurrencyDTO<SuperGroup>;

impl MemberOf<Self> for SuperGroup {}
impl Group for SuperGroup {
    const DESCR: &'static str = "super_group";
    type TopG = Self;

    fn filter_map<FilterMap, FilterMapRef>(
        f: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        CurrenciesMapping::<_, SuperGroupItem, _, _>::with_filter(f.clone())
            .chain(SubGroup::filter_map(SubFilterAdapter::new(f)))
    }

    fn find_map<FindMap>(v: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = Self>,
    {
        group::find_map::<_, SuperGroupItem, _>(v)
            .or_else(|v| {
                group::find_map::<_, SubGroupItem, _>(SubGroupFindAdapter::<SubGroup, _, _>::new(v))
            })
            .map_err(|adapter| adapter.release_super_map())
    }
}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SuperGroupTestC1 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SuperGroupTestC1Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SuperGroupTestC1 {}
impl InPoolWith<SuperGroupTestC2> for SuperGroupTestC1 {}
impl InPoolWith<SuperGroupTestC4> for SuperGroupTestC1 {}
impl InPoolWith<SuperGroupTestC5> for SuperGroupTestC1 {}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SuperGroupTestC2 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SuperGroupTestC2Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SuperGroupTestC2 {}
impl InPoolWith<SuperGroupTestC1> for SuperGroupTestC2 {}
impl InPoolWith<SuperGroupTestC3> for SuperGroupTestC2 {}
impl InPoolWith<SubGroupTestC6> for SuperGroupTestC2 {}
impl InPoolWith<SubGroupTestC10> for SuperGroupTestC2 {}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SuperGroupTestC3 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SuperGroupTestC3Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SuperGroupTestC3 {}
impl InPoolWith<SuperGroupTestC2> for SuperGroupTestC3 {}
impl InPoolWith<SuperGroupTestC5> for SuperGroupTestC3 {}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SuperGroupTestC4 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SuperGroupTestC4Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SuperGroupTestC4 {}
impl InPoolWith<SuperGroupTestC1> for SuperGroupTestC4 {}
impl InPoolWith<SuperGroupTestC3> for SuperGroupTestC4 {}
impl InPoolWith<SuperGroupTestC5> for SuperGroupTestC4 {}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SuperGroupTestC5 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SuperGroupTestC5Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SuperGroupTestC5 {}
impl InPoolWith<SuperGroupTestC1> for SuperGroupTestC5 {}
impl InPoolWith<SuperGroupTestC3> for SuperGroupTestC5 {}
impl InPoolWith<SuperGroupTestC4> for SuperGroupTestC5 {}
impl InPoolWith<SubGroupTestC10> for SuperGroupTestC5 {}

#[derive(Debug, Copy, Clone, Ord, PartialEq, PartialOrd, Eq, Deserialize)]
pub struct SubGroup {}
pub type SubGroupCurrency = CurrencyDTO<SubGroup>;

impl MemberOf<Self> for SubGroup {}
impl MemberOf<SuperGroup> for SubGroup {}
impl Group for SubGroup {
    const DESCR: &'static str = "sub_group";
    type TopG = SuperGroup;

    fn filter_map<FilterMap, FilterMapRef>(
        f: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap>,
    {
        CurrenciesMapping::<_, SubGroupItem, _, _>::with_filter(f)
    }

    fn find_map<FindMap>(v: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = Self>,
    {
        group::find_map::<_, SubGroupItem, _>(v)
    }
}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SubGroupTestC6 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SubGroupTestC6Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SubGroupTestC6 {}
impl InPoolWith<SuperGroupTestC2> for SubGroupTestC6 {}
impl InPoolWith<SubGroupTestC10> for SubGroupTestC6 {}

//Pool pairs: 1:2, 1:4, 2:3, 4:5, 2:6, 2:10, 5:10, 6:10
impl PairsGroup for SubGroupTestC10 {
    type CommonGroup = SuperGroup;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: PairsFindMapT<Pivot = Self>,
    {
        pairs::find_map::<SubGroupTestC10Pairs, _>(f)
    }
}
impl InPoolWith<SuperGroup> for SubGroupTestC10 {}
impl InPoolWith<SuperGroupTestC1> for SubGroupTestC10 {}
impl InPoolWith<SuperGroupTestC2> for SubGroupTestC10 {}
impl InPoolWith<SuperGroupTestC3> for SubGroupTestC10 {}
impl InPoolWith<SuperGroupTestC4> for SubGroupTestC10 {}
impl InPoolWith<SuperGroupTestC5> for SubGroupTestC10 {}
impl InPoolWith<SubGroupTestC6> for SubGroupTestC10 {}

mod impl_ {
    use serde::{Deserialize, Serialize};

    use crate::{CurrencyDTO, CurrencyDef, Definition};

    use super::{SubGroup, SuperGroup};

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC1(CurrencyDTO<SuperGroup>);

    impl CurrencyDef for TestC1 {
        type Group = SuperGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC1 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#1", "ibc/bank_ticker#1", "ibc/dex_ticker#1", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC2(CurrencyDTO<SuperGroup>);

    impl CurrencyDef for TestC2 {
        type Group = SuperGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC2 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#2", "ibc/bank_ticker#2", "ibc/dex_ticker#2", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC3(CurrencyDTO<SuperGroup>);

    impl CurrencyDef for TestC3 {
        type Group = SuperGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC3 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#3", "ibc/bank_ticker#3", "ibc/dex_ticker#3", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC4(CurrencyDTO<SuperGroup>);

    impl CurrencyDef for TestC4 {
        type Group = SuperGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC4 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#4", "ibc/bank_ticker#4", "ibc/dex_ticker#4", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC5(CurrencyDTO<SuperGroup>);

    impl CurrencyDef for TestC5 {
        type Group = SuperGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC5 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#5", "ibc/bank_ticker#5", "ibc/dex_ticker#5", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC6(CurrencyDTO<SubGroup>);

    impl CurrencyDef for TestC6 {
        type Group = SubGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC6 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#6", "ibc/bank_ticker#6", "ibc/dex_ticker#6", 6),
            );
            &DTO
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
    pub struct TestC10(CurrencyDTO<SubGroup>);

    impl CurrencyDef for TestC10 {
        type Group = SubGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            const DTO: CurrencyDTO<<TestC10 as CurrencyDef>::Group> = CurrencyDTO::new(
                &Definition::new("ticker#10", "ibc/bank_ticker#10", "ibc/dex_ticker#10", 6),
            );
            &DTO
        }
    }
}
