use crate::{CurrencyDef, FilterMapT, FindMapT, Group};

/// Member type of a group
///
/// Express a 'member-of' relation of currency *types* in compile-time.
///
/// Not to be confused with [`GroupMember`]!
pub trait MemberOf<G>
where
    G: Group,
{
}

impl<G, C> MemberOf<G> for C
where
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    G: Group,
{
}

/// Member of a group
///
/// Express a 'member-of' relation of currency *values*.
///
/// Intended to facilitate the iteration of group members.
///
/// Not to be confused [`MemberOf`]!
pub trait GroupMember<G>
where
    Self: Sized,
    G: Group,
{
    fn first() -> Option<Self>;

    fn next(&self) -> Option<Self>;

    fn filter_map<FilterMap>(&self, filter_map: &FilterMap) -> Option<FilterMap::Outcome>
    where
        FilterMap: FilterMapT<G>;

    fn find_map<FindMap>(&self, find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = G>;
}
