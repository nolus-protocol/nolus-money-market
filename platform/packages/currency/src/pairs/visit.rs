use crate::{CurrencyDef, Group, InPoolWith};

use super::PairsGroup;

pub trait PairedWithList<Pivot>
where
    Pivot: PairsGroup,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: self::Visitor<Pivot>;
}

impl<Pivot> PairedWithList<Pivot> for ()
where
    Pivot: PairsGroup,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: self::Visitor<Pivot>,
    {
        None
    }
}

impl<Pivot, Head> PairedWithList<Pivot> for (Head,)
where
    Pivot: PairsGroup,
    Head: CurrencyDef + InPoolWith<Pivot> + PairsGroup<CommonGroup = Pivot::CommonGroup>,
    Head::Group: Group<TopG = Pivot::CommonGroup>,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: self::Visitor<Pivot>,
    {
        Some(PairedWith {
            visit: Visitor::visit::<Head>,
            next: <() as PairedWithList<Pivot>>::next::<Visitor>,
        })
    }
}

impl<Pivot, Head, Tail> PairedWithList<Pivot> for (Head, Tail)
where
    Pivot: PairsGroup,
    Head: CurrencyDef + InPoolWith<Pivot> + PairsGroup<CommonGroup = Pivot::CommonGroup>,
    Head::Group: Group<TopG = Pivot::CommonGroup>,
    Tail: PairedWithList<Pivot>,
{
    fn next<Visitor>() -> Option<PairedWith<Pivot, Visitor>>
    where
        Visitor: self::Visitor<Pivot>,
    {
        Some(PairedWith {
            visit: Visitor::visit::<Head>,
            next: Tail::next::<Visitor>,
        })
    }
}

pub trait Visitor<Pivot>
where
    Pivot: PairsGroup,
{
    type Output;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef + InPoolWith<Pivot> + PairsGroup<CommonGroup = Pivot::CommonGroup>,
        C::Group: Group<TopG = Pivot::CommonGroup>;
}

pub struct PairedWith<Pivot, Visitor>
where
    Pivot: PairsGroup,
    Visitor: self::Visitor<Pivot>,
{
    visit: fn(Visitor) -> Visitor::Output,
    next: fn() -> Option<Self>,
}

pub(super) struct MembersIter<Pivot, Visitor>
where
    Pivot: PairsGroup,
    Visitor: self::Visitor<Pivot>,
{
    next: fn() -> Option<PairedWith<Pivot, Visitor>>,
}

impl<Pivot, Visitor> MembersIter<Pivot, Visitor>
where
    Pivot: CurrencyDef + PairsGroup,
    Pivot::Group: Group<TopG = Pivot::CommonGroup>,
    Visitor: self::Visitor<Pivot>,
{
    pub const fn new() -> Self {
        const {
            Self {
                next: Pivot::PairedWith::next::<Visitor>,
            }
        }
    }

    pub fn next(&mut self) -> Option<fn(Visitor) -> Visitor::Output> {
        let visit;

        PairedWith {
            visit,
            next: self.next,
        } = (self.next)()?;

        Some(visit)
    }
}
