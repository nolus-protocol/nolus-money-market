use std::{iter, ops::ControlFlow};

use crate::{CurrencyDef, Group, InPoolWith, PairsFindMap, PairsGroup};

use super::visit::{MembersIter, Visitor};

pub fn find<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    FindMap: PairsFindMap,
    FindMap::Pivot: CurrencyDef,
    <FindMap::Pivot as CurrencyDef>::Group:
        Group<TopG = <FindMap::Pivot as PairsGroup>::CommonGroup>,
{
    let mut members = const { MembersIter::new() };

    let output = iter::from_fn(move || members.next()).try_fold(find_map, |find_map, f| {
        match f(Adapter(find_map)) {
            Ok(output) => ControlFlow::Break(output),
            Err(find_map) => ControlFlow::Continue(find_map),
        }
    });

    match output {
        ControlFlow::Continue(find_map) => Err(find_map),
        ControlFlow::Break(output) => Ok(output),
    }
}

struct Adapter<FindMap>(FindMap)
where
    FindMap: PairsFindMap;

impl<FindMap> Visitor<FindMap::Pivot> for Adapter<FindMap>
where
    FindMap: PairsFindMap,
{
    type Output = Result<FindMap::Outcome, FindMap>;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef
            + InPoolWith<<FindMap as PairsFindMap>::Pivot>
            + PairsGroup<CommonGroup = <FindMap::Pivot as PairsGroup>::CommonGroup>,
        C::Group: Group<TopG = <FindMap::Pivot as PairsGroup>::CommonGroup>,
    {
        self.0.on::<C>(C::dto())
    }
}
