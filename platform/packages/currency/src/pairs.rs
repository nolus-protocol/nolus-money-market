use crate::{CurrencyDTO, CurrencyDef, Group, Matcher, MemberOf, from_symbol_any::InPoolWith};

pub type PairsVisitorResult<Visitor> = <Visitor as PairsVisitor>::Outcome;

pub type MaybePairsVisitorResult<V> = Result<PairsVisitorResult<V>, V>;

pub trait PairsGroup {
    type CommonGroup: Group;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybePairsVisitorResult<V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>;
}

pub trait PairsVisitor
where
    Self: Sized,
{
    type Pivot: PairsGroup;

    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> PairsVisitorResult<Self>
    where
        C: CurrencyDef
            + InPoolWith<Self::Pivot>
            + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
        C::Group: MemberOf<<Self::Pivot as PairsGroup>::CommonGroup>;
}
