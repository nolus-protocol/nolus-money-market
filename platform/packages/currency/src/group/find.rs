use crate::{FindMapT, Group, group::GroupMember};

pub fn find_map<G, GroupMemberImpl, FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    G: Group,
    GroupMemberImpl: GroupMember<G>,
    FindMap: FindMapT<G>,
{
    let mut may_next = GroupMemberImpl::first();
    let mut result = Err(f);
    while let Some(next) = may_next {
        match result {
            Ok(ref _result) => {
                break;
            }
            Err(f) => {
                result = next.find_map(f);
            }
        }
        may_next = next.next();
    }
    result
}
