use crate::{PairsGroup, pairs::FindMapT as PairsFindMapT};

/// Member of a pairs group
///
/// Express a 'member-of' relation of currency *values*.
///
/// Intended to facilitate the iteration of pairs group members.
///
/// Not to be confused [`MemberOf`] nor [`crate::FindMapT`]!
pub trait PairsGroupMember
where
    Self: Sized,
{
    type Group: PairsGroup;

    fn first() -> Option<Self>;

    fn next(&self) -> Option<Self>;

    fn find_map<PairsFindMap>(
        &self,
        find_map: PairsFindMap,
    ) -> Result<PairsFindMap::Outcome, PairsFindMap>
    where
        PairsFindMap: PairsFindMapT<Pivot = Self::Group>;
}
