use crate::pairs::{FindMapT, PairsGroupMember};

pub fn find_map<PairsGroupMemberImpl, FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    PairsGroupMemberImpl: PairsGroupMember,
    FindMap: FindMapT<Pivot = PairsGroupMemberImpl::Group>,
{
    let mut may_next = PairsGroupMemberImpl::first();
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
