use crate::{PairsFindMap, PairsGroupMember};

pub fn find_map<PairsGroupMemberImpl, FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    PairsGroupMemberImpl: PairsGroupMember,
    FindMap: PairsFindMap<Pivot = PairsGroupMemberImpl::Group>,
{
    let mut may_next = PairsGroupMemberImpl::first();
    let mut result = Err(f);
    while let Some(next) = may_next {
        result = if let Err(f) = result {
            next.find_map(f)
        } else {
            break;
        };
        may_next = next.next();
    }
    result
}
