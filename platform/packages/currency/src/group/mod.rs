use std::{borrow::Borrow, fmt::Debug};

use crate::{CurrencyDTO, CurrencyDef, PairsGroup};

use super::AnyVisitor;

pub use self::{
    adapter::{SubFilterAdapter, SubGroupFindAdapter},
    filter::CurrenciesMapping,
    find::find_map,
    member::{GroupMember, MemberOf},
};

mod adapter;
mod filter;
mod find;
mod member;

/// A group of strong typed [`Currency`]-ies
///
/// It is like a collection of types validated statically by the Rust compiler.
/// Since there is no notion of a 'meta-types', the members of a group cannot be iterated over.
/// Instead, we can deal with their mapped values, though.
pub trait Group
where
    Self: Copy + Clone + Debug + Ord + PartialEq + MemberOf<Self> + MemberOf<Self::TopG>,
{
    const DESCR: &'static str;
    type TopG: Group<TopG = Self::TopG>;

    /// Creates an iterator that both filters and maps currencies.
    ///
    /// - type arg: `FilterMapRef` - abstracts how the filter is passed, either by value or &.
    ///
    /// The elements of the returned iterator are produced by the provided functor
    /// mapping a currency to `Some(value)`. A currency for which the functor returns
    /// `None` is skipped.
    fn filter_map<FilterMap, FilterMapRef>(
        f: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone;

    /// Find and map a currency to value
    ///
    /// The first currency for which the [`FindMap`] argument produces [`Ok(mapped_value)`]
    /// stops the iteration and that result is returned.
    /// If there is no such currency, [`Err(v)`] is returned.
    fn find_map<FindMap>(v: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = Self>;
}

pub type MaybeAnyVisitResult<VisitedG, V> = Result<<V as AnyVisitor<VisitedG>>::Outcome, V>;

pub trait FilterMapT {
    type VisitedG: Group;

    type Outcome;

    //TODO consider removing the function argument `def` if the wasm binaries do not become too large
    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <Self::VisitedG as Group>::TopG>,
        C::Group: MemberOf<Self::VisitedG> + MemberOf<<Self::VisitedG as Group>::TopG>;
}

pub trait FindMapT
where
    Self: Sized,
{
    type TargetG: Group;

    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <Self::TargetG as Group>::TopG>,
        C::Group: MemberOf<Self::TargetG> + MemberOf<<Self::TargetG as Group>::TopG>;
}
