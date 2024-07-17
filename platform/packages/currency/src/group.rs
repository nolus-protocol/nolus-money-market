use std::fmt::Debug;

use crate::Currency;

use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult};

pub trait Group: Copy + Clone + Debug + PartialEq + MemberOf<Self> {
    const DESCR: &'static str;

    // Visit this group directly by a visitor
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher<Group = Self>,
        V: AnyVisitor<VisitedG = Self>;

    // Visit this group since it is a member, or a sub-group, of another that is being visited
    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher,
        V: AnyVisitor,
        Self: MemberOf<V::VisitedG> + MemberOf<M::Group>;
}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;

pub trait MemberOf<G>
where
    G: Group,
{
}

impl<G, C> MemberOf<G> for C
where
    C: Currency,
    C::Group: MemberOf<G>,
    G: Group,
{
}
