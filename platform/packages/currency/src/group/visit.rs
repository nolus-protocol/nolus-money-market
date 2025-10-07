use crate::{CurrencyDef, PairsGroup};

use super::{Group, MemberOf};

pub trait MembersList<VisitedGroup>
where
    VisitedGroup: Group,
{
    fn next<Visitor>() -> Option<Member<VisitedGroup, Visitor>>
    where
        Visitor: self::Visitor<VisitedGroup>;
}

impl<VisitedGroup> MembersList<VisitedGroup> for ()
where
    VisitedGroup: Group,
{
    fn next<Visitor>() -> Option<Member<VisitedGroup, Visitor>>
    where
        Visitor: self::Visitor<VisitedGroup>,
    {
        None
    }
}

impl<VisitedGroup, Head> MembersList<VisitedGroup> for (Head,)
where
    VisitedGroup: Group,
    Head: CurrencyDef<Group = VisitedGroup> + PairsGroup<CommonGroup = VisitedGroup::TopG>,
    Head::Group: MemberOf<VisitedGroup> + MemberOf<VisitedGroup::TopG>,
{
    fn next<Visitor>() -> Option<Member<VisitedGroup, Visitor>>
    where
        Visitor: self::Visitor<VisitedGroup>,
    {
        Some(Member {
            visit: Visitor::visit::<Head>,
            next: <() as MembersList<VisitedGroup>>::next::<Visitor>,
        })
    }
}

impl<VisitedGroup, Head, Tail> MembersList<VisitedGroup> for (Head, Tail)
where
    VisitedGroup: Group,
    Head: CurrencyDef<Group = VisitedGroup> + PairsGroup<CommonGroup = VisitedGroup::TopG>,
    Head::Group: MemberOf<VisitedGroup> + MemberOf<VisitedGroup::TopG>,
    Tail: MembersList<VisitedGroup>,
{
    fn next<Visitor>() -> Option<Member<VisitedGroup, Visitor>>
    where
        Visitor: self::Visitor<VisitedGroup>,
    {
        Some(Member {
            visit: Visitor::visit::<Head>,
            next: Tail::next::<Visitor>,
        })
    }
}

pub(super) trait Visitor<VisitedGroup>
where
    VisitedGroup: Group,
{
    type Output;

    fn visit<C>(self) -> Self::Output
    where
        C: CurrencyDef<Group = VisitedGroup> + PairsGroup<CommonGroup = VisitedGroup::TopG>,
        C::Group: MemberOf<VisitedGroup::TopG>;
}

pub(super) struct Member<VisitedGroup, Visitor>
where
    VisitedGroup: Group,
    Visitor: self::Visitor<VisitedGroup>,
{
    visit: fn(Visitor) -> Visitor::Output,
    next: fn() -> Option<Self>,
}

pub(super) struct MembersIter<VisitedGroup, Visitor>
where
    VisitedGroup: Group,
    Visitor: self::Visitor<VisitedGroup>,
{
    next: fn() -> Option<Member<VisitedGroup, Visitor>>,
}

impl<VisitedGroup, Visitor> MembersIter<VisitedGroup, Visitor>
where
    VisitedGroup: Group,
    Visitor: self::Visitor<VisitedGroup>,
{
    pub const fn new() -> Self {
        const {
            Self {
                next: VisitedGroup::Members::next::<Visitor>,
            }
        }
    }

    pub fn next(&mut self) -> Option<fn(Visitor) -> Visitor::Output> {
        let visit;

        Member {
            visit,
            next: self.next,
        } = (self.next)()?;

        Some(visit)
    }
}
