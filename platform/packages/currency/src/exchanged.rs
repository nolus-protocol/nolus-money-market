use crate::{AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, MemberOf};

// pub trait ExchangedFor<C>
// where
//     Self: Currency,
//     C: Currency,
// {
// }
// impl<C> ExchangedFor<AnyCurrency> for C where C: Currency {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct AnyCurrency;

impl Currency for AnyCurrency {}

pub trait InPoolWith<C = AnyCurrency> {
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self, VisitorG = Self>,
        Self: Group + MemberOf<V::VisitorG>;

    fn maybe_visit_member<M, V, TopG>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<TopG>, //TODO may constrain further AnyVisitor::on<C_fiance> where C_fiance: ExchangedFor<C>
        Self: MemberOf<TopG> + MemberOf<V::VisitorG>,
        TopG: Group + MemberOf<V::VisitorG>;
}
