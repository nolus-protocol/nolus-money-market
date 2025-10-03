use crate::{
    CurrencyDef, GroupFilterMap, GroupFindMap,
    group::GroupMember,
    pairs::{FindMapT as PairsFindMapT, PairsGroupMember},
    test::{
        SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1, SuperGroupTestC2,
        SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
    },
};

// ======== START GENERATED CODE =========
pub(super) enum Item {
    SuperGroupTestC1,
    SuperGroupTestC2,
    SuperGroupTestC3,
    SuperGroupTestC4,
    SuperGroupTestC5,
}

impl GroupMember<SuperGroup> for Item {
    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC1)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC1 => Some(Self::SuperGroupTestC2),
            Self::SuperGroupTestC2 => Some(Self::SuperGroupTestC3),
            Self::SuperGroupTestC3 => Some(Self::SuperGroupTestC4),
            Self::SuperGroupTestC4 => Some(Self::SuperGroupTestC5),
            Self::SuperGroupTestC5 => None,
        }
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = SuperGroup>,
    {
        match *self {
            Self::SuperGroupTestC1 => filter_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Self::SuperGroupTestC2 => filter_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
            Self::SuperGroupTestC3 => filter_map.on::<SuperGroupTestC3>(SuperGroupTestC3::dto()),
            Self::SuperGroupTestC4 => filter_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Self::SuperGroupTestC5 => filter_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
        }
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = SuperGroup>,
    {
        match *self {
            Self::SuperGroupTestC1 => find_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Self::SuperGroupTestC2 => find_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
            Self::SuperGroupTestC3 => find_map.on::<SuperGroupTestC3>(SuperGroupTestC3::dto()),
            Self::SuperGroupTestC4 => find_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Self::SuperGroupTestC5 => find_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
        }
    }
}

pub(super) enum SuperGroupTestC1Pairs {
    SuperGroupTestC2,
    SuperGroupTestC4,
    SubGroupTestC10,
}

impl PairsGroupMember for SuperGroupTestC1Pairs {
    type Group = SuperGroupTestC1;

    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC2)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC2 => Some(Self::SuperGroupTestC4),
            Self::SuperGroupTestC4 => Some(Self::SubGroupTestC10),
            Self::SubGroupTestC10 => None,
        }
    }

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
    {
        match *self {
            Self::SuperGroupTestC2 => find_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
            Self::SuperGroupTestC4 => find_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Self::SubGroupTestC10 => find_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }
}

pub(super) enum SuperGroupTestC2Pairs {
    SuperGroupTestC1,
    SuperGroupTestC3,
    SubGroupTestC6,
    SubGroupTestC10,
}

impl PairsGroupMember for SuperGroupTestC2Pairs {
    type Group = SuperGroupTestC2;

    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC1)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC1 => Some(Self::SuperGroupTestC3),
            Self::SuperGroupTestC3 => Some(Self::SubGroupTestC6),
            Self::SubGroupTestC6 => Some(Self::SubGroupTestC10),
            Self::SubGroupTestC10 => None,
        }
    }

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
    {
        match *self {
            Self::SuperGroupTestC1 => find_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Self::SuperGroupTestC3 => find_map.on::<SuperGroupTestC3>(SuperGroupTestC3::dto()),
            Self::SubGroupTestC6 => find_map.on::<SubGroupTestC6>(SubGroupTestC6::dto()),
            Self::SubGroupTestC10 => find_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }
}

pub(super) enum SuperGroupTestC3Pairs {
    SuperGroupTestC2,
}

impl PairsGroupMember for SuperGroupTestC3Pairs {
    type Group = SuperGroupTestC3;

    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC2)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC2 => None,
        }
    }

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
    {
        match *self {
            Self::SuperGroupTestC2 => find_map.on::<SuperGroupTestC2>(SuperGroupTestC2::dto()),
        }
    }
}

pub(super) enum SuperGroupTestC4Pairs {
    SuperGroupTestC1,
    SuperGroupTestC5,
}

impl PairsGroupMember for SuperGroupTestC4Pairs {
    type Group = SuperGroupTestC4;

    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC1)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC1 => Some(Self::SuperGroupTestC5),
            Self::SuperGroupTestC5 => None,
        }
    }

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
    {
        match *self {
            Self::SuperGroupTestC1 => find_map.on::<SuperGroupTestC1>(SuperGroupTestC1::dto()),
            Self::SuperGroupTestC5 => find_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
        }
    }
}

pub(super) enum SuperGroupTestC5Pairs {
    SuperGroupTestC4,
    SuperGroupTestC5,
    SubGroupTestC10,
}

impl PairsGroupMember for SuperGroupTestC5Pairs {
    type Group = SuperGroupTestC5;

    fn first() -> Option<Self> {
        Some(Self::SuperGroupTestC4)
    }

    fn next(&self) -> Option<Self> {
        match self {
            Self::SuperGroupTestC4 => Some(Self::SuperGroupTestC5),
            Self::SuperGroupTestC5 => Some(Self::SubGroupTestC10),
            Self::SubGroupTestC10 => None,
        }
    }

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
    {
        match *self {
            Self::SuperGroupTestC4 => find_map.on::<SuperGroupTestC4>(SuperGroupTestC4::dto()),
            Self::SuperGroupTestC5 => find_map.on::<SuperGroupTestC5>(SuperGroupTestC5::dto()),
            Self::SubGroupTestC10 => find_map.on::<SubGroupTestC10>(SubGroupTestC10::dto()),
        }
    }
}
// ======== END GENERATED CODE =========

#[cfg(test)]
mod test {

    use crate::{
        CurrencyDef, Group, Tickers,
        matcher::symbol_matcher,
        test::{
            FindCurrencyBySymbol, SubGroupTestC6, SubGroupTestC10, SuperGroup, SuperGroupTestC1,
            SuperGroupTestC2, SuperGroupTestC3, SuperGroupTestC4, SuperGroupTestC5,
            filter::{Dto, FindByTicker},
        },
    };

    #[test]
    fn enumerate_all() {
        let filter = Dto::default();
        let mut iter = SuperGroup::filter_map(filter);

        assert_eq!(Some(SuperGroupTestC1::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC2::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC4::dto()), iter.next().as_ref());
        assert_eq!(Some(SuperGroupTestC5::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC6::dto().into_super_group()), iter.next());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn skip_some() {
        let filter = FindByTicker::new(SuperGroupTestC3::ticker(), SubGroupTestC10::ticker());
        let mut iter = SuperGroup::filter_map(filter);
        assert_eq!(Some(SuperGroupTestC3::dto()), iter.next().as_ref());
        assert_eq!(Some(SubGroupTestC10::dto().into_super_group()), iter.next());
        assert_eq!(None, iter.next().as_ref());
    }

    #[test]
    fn find() {
        find_ok::<SuperGroupTestC1>();
        find_ok::<SuperGroupTestC2>();
        find_ok::<SuperGroupTestC3>();
        find_ok::<SuperGroupTestC4>();
        find_ok::<SuperGroupTestC5>();
        find_ok::<SubGroupTestC6>();
        find_ok::<SubGroupTestC10>();
        find_nok("unknown ticker");
    }

    #[track_caller]
    fn find_ok<C>()
    where
        C: CurrencyDef,
    {
        let matcher = symbol_matcher::<Tickers<SuperGroup>>(C::ticker());
        assert_eq!(
            C::dto(),
            &SuperGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).unwrap()
        );
    }

    fn find_nok(ticker: &str) {
        let matcher = symbol_matcher::<Tickers<SuperGroup>>(ticker);
        assert!(SuperGroup::find_map(FindCurrencyBySymbol::with_matcher(matcher)).is_err());
    }
}
