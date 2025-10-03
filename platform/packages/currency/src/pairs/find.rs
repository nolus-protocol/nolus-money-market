use crate::{CurrencyDef, Group, InPoolWith, PairsFindMapT, PairsGroup};

use super::visit::{MembersIter, Visitor};

pub fn find<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
where
    FindMap: PairsFindMapT,
    FindMap::Pivot: CurrencyDef,
    <FindMap::Pivot as CurrencyDef>::Group:
        Group<TopG = <FindMap::Pivot as PairsGroup>::CommonGroup>,
{
    let mut members = const { MembersIter::new() };

    let output =
        std::iter::from_fn(move || members.next()).try_fold(find_map, |find_map, f| {
            match f(Adapter(find_map)) {
                Ok(output) => std::ops::ControlFlow::Break(output),
                Err(find_map) => std::ops::ControlFlow::Continue(find_map),
            }
        });

    match output {
        std::ops::ControlFlow::Continue(find_map) => Err(find_map),
        std::ops::ControlFlow::Break(output) => Ok(output),
    }
}

struct Adapter<FindMap>(FindMap)
where
    FindMap: PairsFindMapT;

impl<FindMap> Visitor<FindMap::Pivot> for Adapter<FindMap>
where
    FindMap: PairsFindMapT,
{
    type Output = Result<FindMap::Outcome, FindMap>;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef
            + InPoolWith<<FindMap as PairsFindMapT>::Pivot>
            + PairsGroup<CommonGroup = <FindMap::Pivot as PairsGroup>::CommonGroup>,
        C::Group: Group<TopG = <FindMap::Pivot as PairsGroup>::CommonGroup>,
    {
        self.0.on::<C>(C::dto())
    }
}
