use std::fmt::Debug;

use crate::CurrencyDef;

use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult};

pub trait Group
where
    Self: Copy + Clone + Debug + Ord + PartialEq + MemberOf<Self>,
    Self: MemberOf<Self::TopG>,
{
    const DESCR: &'static str;
    type TopG: Group;

    // Visit this group directly by a visitor
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>,
        Self: Group<TopG = Self>;

    // Visit this group with a super-group visitor
    fn maybe_visit_super_visitor<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>;

    // Visit this group since it is a member, or a sub-group, of another that is being visited
    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    //TODO consider factor it out, called only from within Group::maybe_visit
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>;
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
