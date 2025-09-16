use crate::{CurrencyDTO, CurrencyDef, Group, MemberOf, visit_any::InPoolWith};

#[cfg(any(test, feature = "testing"))]
pub use find::find_map;
#[cfg(any(test, feature = "testing"))]
pub use member::PairsGroupMember;

//TODO remove once generated production pairs show up
#[cfg(any(test, feature = "testing"))]
mod find;
//TODO the same
#[cfg(any(test, feature = "testing"))]
mod member;

pub type MaybePairsVisitorResult<V> = Result<<V as PairsVisitor>::Outcome, V>;

/// A group of strong typed [`Currency`]-ies that form with [`self`] valid swap pools on the DEX.
///
/// For each currency *C*, a swap pool ([`self`], *C*`) or (*C*, [`self`]) exists on the Dex.
///
/// The collection of pair types are validated statically by the Rust compiler.
/// Since there is no notion of a 'meta-types', the members of a group cannot be iterated over.
/// Instead, we deal with their mapped values.
pub trait PairsGroup {
    type CommonGroup: Group;

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
