use crate::{Group, GroupFindMapT, GroupMember};

pub fn find_map<G, GroupMemberImpl, FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    G: Group,
    GroupMemberImpl: GroupMember<G>,
    FindMap: GroupFindMapT<TargetG = G>,
{
    let mut may_next = GroupMemberImpl::first();
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
