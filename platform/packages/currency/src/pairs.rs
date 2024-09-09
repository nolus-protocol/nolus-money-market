use crate::{from_symbol_any::InPoolWith, CurrencyDTO, CurrencyDef, Group, Matcher, MemberOf};

pub type PairsVisitorResult<Visitor> =
    Result<<Visitor as PairsVisitor>::Output, <Visitor as PairsVisitor>::Error>;

pub type MaybePairsVisitorResult<V> = Result<PairsVisitorResult<V>, V>;

pub trait PairsGroup {
    type CommonGroup: Group;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self, VisitedG = Self::CommonGroup>;
}

pub trait PairsVisitor
where
    Self: Sized,
{
    type Pivot;
    type VisitedG: Group;

    type Output;
    type Error;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> PairsVisitorResult<Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = Self::VisitedG> + InPoolWith<Self::Pivot>, // TODO consider moving the PairsGroup trait bound to AnyVisitor to drop `impl PairsGroup for <Groups>`
        C::Group: MemberOf<Self::VisitedG>;
}
