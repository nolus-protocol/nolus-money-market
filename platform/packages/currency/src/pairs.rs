use crate::{CurrencyDTO, CurrencyDef, Group, Matcher, MemberOf};

pub type PairsVisitorResult<Visitor> =
    Result<<Visitor as PairsVisitor>::Output, <Visitor as PairsVisitor>::Error>;

pub type MaybePairsVisitorResult<V> = Result<PairsVisitorResult<V>, V>;

pub trait PairsGroup {
    type CommonGroup: Group;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<VisitedG = Self::CommonGroup>;
}

pub trait PairsVisitor
where
    Self: Sized,
{
    type VisitedG: Group;

    type Output;
    type Error;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> PairsVisitorResult<Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = Self::VisitedG>,
        C::Group: Group + MemberOf<Self::VisitedG>;
}
