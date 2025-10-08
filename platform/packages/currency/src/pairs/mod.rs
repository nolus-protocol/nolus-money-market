use crate::{CurrencyDTO, CurrencyDef, Group, MemberOf, visit_any::InPoolWith};

pub use self::find::find;
pub(crate) use self::visit::{PairedWith, PairedWithList, Visitor};

mod find;
mod visit;

pub type MaybePairsVisitorResult<V> = Result<<V as PairsVisitor>::Outcome, V>;

/// A group of strong typed [`Currency`]-ies that form with [`self`] valid swap pools on the DEX.
///
/// For each currency *C*, a swap pool ([`self`], *C*`) or (*C*, [`self`]) exists on the Dex.
pub trait PairsGroup
where
    Self: Sized,
{
    type CommonGroup: Group<TopG = Self::CommonGroup>;

    type PairedWith: PairedWithList<Self>;

    fn find_map<FindMapImpl>(f: FindMapImpl) -> Result<FindMapImpl::Outcome, FindMapImpl>
    where
        FindMapImpl: FindMap<Pivot = Self>;
}

pub trait FindMap
where
    Self: Sized,
{
    type Pivot: PairsGroup;

    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef
            + InPoolWith<Self::Pivot>
            + PairsGroup<CommonGroup = <Self::Pivot as PairsGroup>::CommonGroup>,
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
