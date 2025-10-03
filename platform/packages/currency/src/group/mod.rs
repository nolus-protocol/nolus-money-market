#![warn(unused, warnings)]

use std::{borrow::Borrow, fmt::Debug, iter, marker::PhantomData, ops::ControlFlow};

use crate::{CurrencyDTO, CurrencyDef, PairsGroup};

use super::AnyVisitor;

use self::visit_members::{CurrencyDefVisitor, MembersIter, MembersList};
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
mod visit_members;

/// A group of strong typed [`Currency`]-ies
///
/// It is like a collection of types validated statically by the Rust compiler.
pub trait Group
where
    Self: Copy + Clone + Debug + Ord + PartialEq + MemberOf<Self> + MemberOf<Self::TopG>,
{
    const DESCR: &'static str;

    type TopG: Group<TopG = Self::TopG>;

    type Members: MembersList<Self>;

    /// Creates an iterator that both filters and maps currencies.
    ///
    /// - type arg: `FilterMapRef` - abstracts how the filter is passed, either by value or &.
    ///
    /// The elements of the returned iterator are produced by the provided functor
    /// mapping a currency to `Some(value)`. A currency for which the functor returns
    /// `None` is skipped.
    fn filter_map<FilterMap, FilterMapRef>(
        filter_map: FilterMapRef,
    ) -> impl Iterator<Item = FilterMap::Outcome>
    where
        FilterMap: FilterMapT<VisitedG = Self>,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        self::filter_map(filter_map)
    }

    /// Find and map a currency to value
    ///
    /// The first currency for which the [`FindMap`] argument produces [`Ok(mapped_value)`]
    /// stops the iteration and that result is returned.
    /// If there is no such currency, [`Err(v)`] is returned.
    fn find_map<FindMap>(find_map: FindMap) -> Result<FindMap::Outcome, FindMap>
    where
        FindMap: FindMapT<TargetG = Self>,
    {
        struct Adapter<FindMap>(FindMap)
        where
            FindMap: FindMapT;

        impl<FindMap> CurrencyDefVisitor<FindMap::TargetG> for Adapter<FindMap>
        where
            FindMap: FindMapT,
        {
            type Output = Result<FindMap::Outcome, FindMap>;

            fn visit<C>(self) -> Self::Output
            where
                C: CurrencyDef + PairsGroup<CommonGroup = <FindMap::TargetG as Group>::TopG>,
                C::Group: MemberOf<FindMap::TargetG> + MemberOf<<FindMap::TargetG as Group>::TopG>,
            {
                self.0.on::<C>(C::dto())
            }
        }

        let mut members = MembersIter::<Self, _>::default();

        let output =
            iter::from_fn(move || members.next()).try_fold(find_map, move |find_map, visit| {
                match visit(Adapter(find_map)) {
                    Ok(output) => ControlFlow::Break(output),
                    Err(find_map) => ControlFlow::Continue(find_map),
                }
            });

        match output {
            ControlFlow::Continue(find_map) => Err(find_map),
            ControlFlow::Break(output) => Ok(output),
        }
    }
}

fn filter_map<G, FilterMap, FilterMapRef>(
    filter_map: FilterMapRef,
) -> impl Iterator<Item = FilterMap::Outcome>
where
    G: Group,
    FilterMap: FilterMapT<VisitedG = G>,
    FilterMapRef: Borrow<FilterMap> + Clone,
{
    struct CurrencyAdapter<FilterMap, FilterMapRef>(FilterMapRef, PhantomData<FilterMap>)
    where
        FilterMap: FilterMapT,
        FilterMapRef: Borrow<FilterMap> + Clone;

    impl<FilterMap, FilterMapRef> CurrencyDefVisitor<FilterMap::VisitedG>
        for CurrencyAdapter<FilterMap, FilterMapRef>
    where
        FilterMap: FilterMapT,
        FilterMapRef: Borrow<FilterMap> + Clone,
    {
        type Output = Option<FilterMap::Outcome>;

        fn visit<C>(self) -> Self::Output
        where
            C: CurrencyDef + PairsGroup<CommonGroup = <FilterMap::VisitedG as Group>::TopG>,
            C::Group:
                MemberOf<FilterMap::VisitedG> + MemberOf<<FilterMap::VisitedG as Group>::TopG>,
        {
            self.0.borrow().on::<C>(C::dto())
        }
    }

    let mut members = const { MembersIter::<G, _>::new() };

    iter::from_fn(move || members.next())
        .filter_map(move |visit| visit(CurrencyAdapter(filter_map.clone(), PhantomData)))
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
