use currency::{GroupFilterMapT, GroupFindMapT};

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
        FilterMap: GroupFilterMapT<VisitedG = OnlyGroup>,
    {
        match *self {}
    }

    fn find_map<FindMap>(&self, _: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: GroupFindMapT<TargetG = OnlyGroup>,
    {
        match *self {}
    }
}
