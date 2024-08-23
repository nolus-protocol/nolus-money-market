use std::fmt::Debug;

use crate::{
    exchanged::{AnyCurrency, InPoolWith},
    CurrencyDef,
};

use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult};

pub trait Group:
    Copy + Clone + Debug + Ord + PartialEq + MemberOf<Self> + InPoolWith<AnyCurrency>
{
    const DESCR: &'static str;

    // Visit this group directly by a visitor
    fn maybe_visit<PivotC, M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
        Self: InPoolWith<PivotC>;

    // Visit this group with a super-group visitor
    fn maybe_visit_super_visitor<PivotC, M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = TopG>,
        Self: MemberOf<TopG> + InPoolWith<PivotC>,
        TopG: Group;

    // Visit this group since it is a member, or a sub-group, of another that is being visited
    fn maybe_visit_member<PivotC, M, V, TopG>(
        matcher: &M,
        visitor: V,
    ) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG, VisitorG = TopG>,
        Self: MemberOf<TopG> + InPoolWith<PivotC>,
        TopG: Group;
}

pub type MaybeAnyVisitResult<VisitedG, V> = Result<AnyVisitorResult<VisitedG, V>, V>;

pub trait MemberOf<G>
where
    G: Group,
{
}

impl<G, C> MemberOf<G> for C
where
    C: CurrencyDef,
    C::Group: MemberOf<G>,
    G: Group,
{
}
