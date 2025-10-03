use crate::{PairsFindMap, PairsGroup};

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

    fn find_map<PairsFindMapImpl>(
        &self,
        find_map: PairsFindMapImpl,
    ) -> Result<PairsFindMapImpl::Outcome, PairsFindMapImpl>
    where
        PairsFindMapImpl: PairsFindMap<Pivot = Self::Group>;
}
