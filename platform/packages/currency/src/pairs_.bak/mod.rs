use crate::{visit_any::InPoolWith, CurrencyDTO, CurrencyDef, Group, Matcher, MemberOf};

pub type MaybePairsVisitorResult<V> = Result<<V as PairsVisitor>::Outcome, V>;

pub trait PairsGroup {
    type CommonGroup: Group;

    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> Result<V::Outcome, V>
    where
        M: Matcher,
        V: PairsVisitor<Pivot = Self>;

    fn find_map<FindMap>(f: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<Pivot = Self>;
}

pub trait FindMapT
where
    Self: Sized,
{
    type Pivot: PairsGroup;

    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
        C::Group: MemberOf<<Self::Pivot as PairsGroup>::CommonGroup>;
}

pub trait PairsVisitor
where
    Self: Sized,
{
    type Pivot: PairsGroup;

    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Self::Outcome
    where
        C: CurrencyDef
            + InPoolWith<Self::Pivot>
            + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
        C::Group: MemberOf<<Self::Pivot as PairsGroup>::CommonGroup>;
}
