use std::{borrow::Borrow, iter, marker::PhantomData};

use crate::{CurrencyDef, Group, GroupFilterMap, MemberOf, PairsGroup};

use super::visit::{MembersIter, Visitor};

pub fn non_recursive<VisitedGroup, FilterMap, FilterMapRef>(
    filter_map: FilterMapRef,
) -> impl Iterator<Item = FilterMap::Outcome>
where
    VisitedGroup: Group,
    FilterMap: GroupFilterMap<VisitedG = VisitedGroup>,
    FilterMapRef: Borrow<FilterMap> + Clone,
{
    let mut members = const { MembersIter::new() };

    iter::from_fn(move || members.next())
        .filter_map(move |visit| visit(Adapter(filter_map.clone(), PhantomData)))
}

struct Adapter<FilterMap, FilterMapRef>(FilterMapRef, PhantomData<FilterMap>)
where
    FilterMap: GroupFilterMap,
    FilterMapRef: Borrow<FilterMap> + Clone;

impl<FilterMap, FilterMapRef> Visitor<FilterMap::VisitedG> for Adapter<FilterMap, FilterMapRef>
where
    FilterMap: GroupFilterMap,
    FilterMapRef: Borrow<FilterMap> + Clone,
{
    type Output = Option<FilterMap::Outcome>;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <FilterMap::VisitedG as Group>::TopG>,
        C::Group: MemberOf<FilterMap::VisitedG> + MemberOf<<FilterMap::VisitedG as Group>::TopG>,
    {
        self.0.borrow().on::<C>(C::dto())
    }
}
