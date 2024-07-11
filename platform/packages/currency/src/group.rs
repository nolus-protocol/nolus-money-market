use super::{matcher::Matcher, AnyVisitor, AnyVisitorResult};

pub trait Group: PartialEq {
    const DESCR: &'static str;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
    where
        M: Matcher + ?Sized,
        V: AnyVisitor;
}

pub type MaybeAnyVisitResult<V> = Result<AnyVisitorResult<V>, V>;
