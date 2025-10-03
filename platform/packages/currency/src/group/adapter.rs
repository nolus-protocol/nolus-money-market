use std::{borrow::Borrow, marker::PhantomData};

use crate::{CurrencyDTO, CurrencyDef, FilterMapT, FindMapT, Group, MemberOf, PairsGroup};

/// Adapter of [`FilterMapT<G::TopG>`] to [`FilterMapT<G>`]
///
/// Aimed for use in super group 'filter_map' implementations
pub struct SubFilterAdapter<G, FilterMap, FilterMapRef>(
    PhantomData<G>,
    FilterMapRef,
    PhantomData<FilterMap>,
);

impl<G, FilterMap, FilterMapRef> SubFilterAdapter<G, FilterMap, FilterMapRef> {
    pub fn new(f: FilterMapRef) -> Self {
        Self(PhantomData, f, PhantomData)
    }
}

impl<G, FilterMap, FilterMapRef> Clone for SubFilterAdapter<G, FilterMap, FilterMapRef>
where
    FilterMapRef: Clone,
{
    fn clone(&self) -> Self {
        Self {
            1: self.1.clone(),
            ..*self
        }
    }
}

impl<G, FilterMap, FilterMapRef> FilterMapT for SubFilterAdapter<G, FilterMap, FilterMapRef>
where
    G: Group,
    FilterMap: FilterMapT<VisitedG = G::TopG>,
    FilterMapRef: Borrow<FilterMap>,
{
    type VisitedG = G;

    type Outcome = FilterMap::Outcome;

    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <G as Group>::TopG>,
        C::Group: MemberOf<<G as Group>::TopG>,
    {
        self.1.borrow().on::<C>(def)
    }
}

/// Adapter of [`FindMapT<G::TopG>`] to [`FindMapT<G>`]
///
/// Aimed for use in super group 'find_map' implementations
pub struct SubGroupFindAdapter<G, FindMap>(PhantomData<G>, FindMap);

impl<G, FindMap> SubGroupFindAdapter<G, FindMap> {
    pub fn new(f: FindMap) -> Self {
        Self(PhantomData, f)
    }
}

impl<G, FindMap> SubGroupFindAdapter<G, FindMap>
where
    G: Group,
    FindMap: FindMapT<TargetG = G::TopG>,
{
    pub fn release_super_map(self) -> FindMap {
        self.1
    }
}

impl<G, FindMap> FindMapT for SubGroupFindAdapter<G, FindMap>
where
    G: Group,
    FindMap: FindMapT<TargetG = G::TopG>,
{
    type TargetG = G;
    type Outcome = FindMap::Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <G as Group>::TopG>,
        C::Group: MemberOf<G> + MemberOf<<G as Group>::TopG>,
    {
        self.1.on::<C>(def).map_err(Self::new)
    }
}
