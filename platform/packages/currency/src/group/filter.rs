use std::{borrow::Borrow, iter, marker::PhantomData};

use crate::{CurrencyDef, FilterMapT, Group, GroupMember, MemberOf, PairsGroup};

use super::visit::{Visitor, MembersIter};

pub fn non_recursive<VisitedGroup, FilterMap, FilterMapRef>(
    filter_map: FilterMapRef,
) -> impl Iterator<Item = FilterMap::Outcome>
where
    VisitedGroup: Group,
    FilterMap: FilterMapT<VisitedG = VisitedGroup>,
    FilterMapRef: Borrow<FilterMap> + Clone,
{
    struct Adapter<FilterMap, FilterMapRef>(FilterMapRef, PhantomData<FilterMap>)
    where
        FilterMap: FilterMapT,
        FilterMapRef: Borrow<FilterMap> + Clone;

    impl<FilterMap, FilterMapRef> Visitor<FilterMap::VisitedG>
        for Adapter<FilterMap, FilterMapRef>
    where
        FilterMap: FilterMapT,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        type Output = Option<FilterMap::Outcome>;

        fn visit<C>(self) -> Self::Output
        where
            C: CurrencyDef + PairsGroup<CommonGroup = <FilterMap::VisitedG as Group>::TopG>,
            C::Group:
                MemberOf<FilterMap::VisitedG> + MemberOf<<FilterMap::VisitedG as Group>::TopG>,
        {
            self.0.borrow().on::<C>(C::dto())
        }
    }

    let mut members = const { MembersIter::new() };

    iter::from_fn(move || members.next())
        .filter_map(move |visit| visit(Adapter(filter_map.clone(), PhantomData)))
}

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
            result = current.filter_map(self.f.borrow());
            self.next = current.next();

            if result.is_some() {
                break;
            }
        }
        result
    }
}
