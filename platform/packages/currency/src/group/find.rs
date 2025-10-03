use std::{iter, ops::ControlFlow};

use crate::{CurrencyDef, FindMapT, Group, MemberOf, PairsGroup};

use super::visit::{MembersIter, Visitor};

pub fn non_recursive<VisitedGroup, FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    VisitedGroup: Group,
    FindMap: FindMapT<TargetG = VisitedGroup>,
{
    let mut members = const { MembersIter::new() };

    let output = iter::from_fn(move || members.next()).try_fold(
        find_map,
        move |find_map, visit| match visit(Adapter(find_map)) {
            Ok(output) => ControlFlow::Break(output),
            Err(find_map) => ControlFlow::Continue(find_map),
        },
    );

    match output {
        ControlFlow::Continue(find_map) => Err(find_map),
        ControlFlow::Break(output) => Ok(output),
    }
}

struct Adapter<FindMap>(FindMap)
where
    FindMap: FindMapT;

impl<FindMap> Visitor<FindMap::TargetG> for Adapter<FindMap>
where
    FindMap: FindMapT,
{
    type Output = Result<FindMap::Outcome, FindMap>;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <FindMap::TargetG as Group>::TopG>,
        C::Group: MemberOf<FindMap::TargetG> + MemberOf<<FindMap::TargetG as Group>::TopG>,
    {
        self.0.on::<C>(C::dto())
    }
}
