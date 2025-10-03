use currency::{GroupFindMap, GroupFilterMap};

use super::Group as OnlyGroup;

pub(super) enum GroupMember {}

impl currency::GroupMember<OnlyGroup> for GroupMember {
    fn first() -> Option<Self> {
        None
    }

    fn next(&self) -> Option<Self> {
        match *self {}
    }

    fn filter_map<FilterMap>(&self, _: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: GroupFilterMap<VisitedG = OnlyGroup>,
    {
        match *self {}
    }

    fn find_map<FindMap>(&self, _: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMap<TargetG = OnlyGroup>,
    {
        match *self {}
    }
}
