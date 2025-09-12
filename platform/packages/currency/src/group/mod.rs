use std::{borrow::Borrow, fmt::Debug};

use crate::{CurrencyDTO, CurrencyDef, PairsGroup};

use super::{AnyVisitor, matcher::Matcher};

pub use filter::CurrenciesMapping;
pub use find::find_map;
pub use member::{GroupMember, MemberOf};

#[cfg(any(test, feature = "testing"))]
pub use self::adapter::{SubFilterAdapter, SubGroupFindAdapter};

// to uncomment once a composite group in production shows up
#[cfg(any(test, feature = "testing"))]
mod adapter;
mod filter;
mod find;
mod member;

pub trait Group
where
    Self: Copy + Clone + Debug + Ord + PartialEq + MemberOf<Self>,
    Self: MemberOf<Self::TopG>,
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
        FilterMap: FilterMapT<Self>,
        FilterMapRef: Borrow<FilterMap> + Clone;

    fn find_map<FindMap>(v: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<Self>;

    // TODO it seems this could be taken out from here into a simple algo-function
    // Visit this group directly by a visitor
    fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        M: Matcher,
        V: AnyVisitor<Self>;

    // TODO it seems this could be taken out from here into a simple algo-function
    // Visit this group since it is a member, or a sub-group, of another that is being visited
    fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<Self::TopG, V>
    where
        M: Matcher,
        V: AnyVisitor<Self::TopG>;
}

pub type MaybeAnyVisitResult<VisitedG, V> = Result<<V as AnyVisitor<VisitedG>>::Outcome, V>;

pub trait FilterMapT<VisitedG>
where
    VisitedG: Group,
{
    type Outcome;

    //TODO consider removing the function argument `def` if the wasm binaries do not become too large
    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>;
}

pub trait FindMapT<VisitedG>
where
    Self: Sized,
    VisitedG: Group,
{
    type Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = VisitedG::TopG>,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>;
}
