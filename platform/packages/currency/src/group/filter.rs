use std::{borrow::Borrow, marker::PhantomData};

use crate::{FilterMapT, Group, group::GroupMember};

/// Iterator over group currency types mapped to some values
pub struct CurrenciesMapping<Group, GroupMember, FilterMap, FilterMapRef> {
    _g_type: PhantomData<Group>,
    next: Option<GroupMember>,
    f: FilterMapRef,
    _f_type: PhantomData<FilterMap>,
}

impl<GroupImpl, GroupMemberImpl, FilterMap, FilterMapRef>
    CurrenciesMapping<GroupImpl, GroupMemberImpl, FilterMap, FilterMapRef>
where
    GroupImpl: Group,
    GroupMemberImpl: GroupMember<GroupImpl>,
    FilterMap: FilterMapT<VisitedG = GroupImpl>,
    FilterMapRef: Borrow<FilterMap>,
{
    pub fn with_filter(f: FilterMapRef) -> Self {
        Self {
            _g_type: PhantomData,
            next: GroupMemberImpl::first(),
            f,
            _f_type: PhantomData,
        }
    }
}

impl<GroupImpl, GroupMemberImpl, FilterMap, FilterMapRef> Iterator
    for CurrenciesMapping<GroupImpl, GroupMemberImpl, FilterMap, FilterMapRef>
where
    GroupImpl: Group,
    GroupMemberImpl: GroupMember<GroupImpl>,
    FilterMap: FilterMapT<VisitedG = GroupImpl>,
    FilterMapRef: Borrow<FilterMap>,
{
    type Item = FilterMap::Outcome;

    fn next(&mut self) -> Option<Self::Item> {
        let mut result = None;
        while let Some(ref current) = self.next {
            if result.is_some() {
                break;
            }

            result = current.filter_map(self.f.borrow());
            self.next = current.next();
        }
        result
    }
}
