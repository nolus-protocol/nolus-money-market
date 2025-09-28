use currency::{CurrencyDef as _, GroupFilterMapT, GroupFindMapT};

use self::definitions::Lpn;

use super::Group as LpnGroup;

pub(super) struct GroupMember;

impl currency::GroupMember<LpnGroup> for GroupMember {
    fn first() -> Option<Self> {
        Some(Self)
    }

    fn next(&self) -> Option<Self> {
        let Self {} = self;

        None
    }

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: GroupFilterMapT<VisitedG = LpnGroup>,
    {
        let Self {} = self;

        filter_map.on::<Lpn>(Lpn::dto())
    }

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMapT<TargetG = LpnGroup>,
    {
        let Self {} = self;

        find_map.on::<Lpn>(Lpn::dto())
    }
}

pub(super) mod definitions {
    use serde::{Deserialize, Serialize};

    use currency::{CurrencyDTO, CurrencyDef, Definition, InPoolWith};

    use crate::{
        lease::{LeaseC2, LeaseC7},
        native::Nls,
    };

    use super::LpnGroup;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
    #[serde(deny_unknown_fields, rename_all = "snake_case")]
    pub struct Lpn(CurrencyDTO<LpnGroup>);

    impl CurrencyDef for Lpn {
        type Group = LpnGroup;

        fn dto() -> &'static CurrencyDTO<Self::Group> {
            &const {
                CurrencyDTO::new(
                    const { &Definition::new("LPN", "ibc/bank_LPN", "ibc/dex_LPN", 6) },
                )
            }
        }
    }

    impl InPoolWith<LeaseC2> for Lpn {}

    impl InPoolWith<LeaseC7> for Lpn {}

    impl InPoolWith<Nls> for Lpn {}
}
