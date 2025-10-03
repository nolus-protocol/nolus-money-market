use currency::{CurrencyDef, GroupFilterMap, FindMapT};

use self::definitions::Nls;

use super::Group as NativeGroup;

pub(super) struct GroupMember;

impl currency::GroupMember<NativeGroup> for GroupMember {
    fn first() -> Option<Self> {
        Some(Self)
    }

    fn next(&self) -> Option<Self> {
        let Self {} = self;

        None
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = NativeGroup>,
    {
        let Self {} = self;

        filter_map.on::<Nls>(Nls::dto())
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = NativeGroup>,
    {
        find_map.on::<Nls>(Nls::dto())
    }
}

pub(super) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{
        CurrencyDTO, CurrencyDef, Definition, InPoolWith, PairsFindMapT, PairsGroup,
        PairsGroupMember, pairs_find_map,
    };

    use crate::{lease::LeaseC5, lpn::Lpn, payment::Group as PaymentGroup};

    use super::NativeGroup;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct Nls(CurrencyDTO<NativeGroup>);

    impl CurrencyDef for Nls {
        type Group = NativeGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const { CurrencyDTO::new(const { &Definition::new("NLS", "unls", "ibc/dex_NLS", 6) }) }
        }
    }

    impl PairsGroup for Nls {
        type CommonGroup = PaymentGroup;

        fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
        where
            FindMap: PairsFindMapT<Pivot = Self>,
        {
            struct Pairs;

            impl PairsGroupMember for Pairs {
                type Group = Nls;

                fn first() -> Option<Self> {
                    Some(Self)
                }

                fn next(&self) -> Option<Self> {
                    None
                }

                fn find_map<PairsFindMap>(
                    &self,
                    find_map: PairsFindMap,
                ) -> Result<PairsFindMap::Outcome, PairsFindMap>
                where
                    PairsFindMap: PairsFindMapT<Pivot = Self::Group>,
                {
                    find_map.on::<Lpn>(<Lpn>::dto())
                }
            }

            pairs_find_map::<Pairs, _>(find_map)
        }
    }

    impl InPoolWith<LeaseC5> for Nls {}
}
