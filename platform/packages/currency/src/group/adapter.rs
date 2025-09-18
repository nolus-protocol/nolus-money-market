use std::{borrow::Borrow, marker::PhantomData};

use crate::{CurrencyDTO, CurrencyDef, FilterMapT, FindMapT, Group, MemberOf, PairsGroup};

/// Adapter of [`FilterMapT<SuperG>`] to [`FilterMapT<G>`]
///
/// Aimed for use in super group 'filter_map' implementations
pub struct SubFilterAdapter<G, SuperG, FilterMap, FilterMapRef>(
    PhantomData<G>,
    PhantomData<SuperG>,
    FilterMapRef,
    PhantomData<FilterMap>,
);

impl<G, SuperG, FilterMap, FilterMapRef> SubFilterAdapter<G, SuperG, FilterMap, FilterMapRef> {
    pub fn new(f: FilterMapRef) -> Self {
        Self(
            PhantomData::<G>,
            PhantomData::<SuperG>,
            f,
            PhantomData::<FilterMap>,
        )
    }
}

impl<G, SuperG, FilterMap, FilterMapRef> Clone
    for SubFilterAdapter<G, SuperG, FilterMap, FilterMapRef>
where
    FilterMapRef: Clone,
{
    fn clone(&self) -> Self {
        Self {
            2: self.2.clone(),
            ..*self
        }
    }
}

impl<G, SuperG, FilterMap, FilterMapRef> FilterMapT
    for SubFilterAdapter<G, SuperG, FilterMap, FilterMapRef>
where
    G: Group<TopG = SuperG>,
    SuperG: Group<TopG = G::TopG>,
    FilterMap: FilterMapT<VisitedG = SuperG>,
    FilterMapRef: Borrow<FilterMap>,
{
    type VisitedG = G;

    type Outcome = FilterMap::Outcome;

    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <G as Group>::TopG>,
        C::Group: MemberOf<<G as Group>::TopG>,
    {
        self.2.borrow().on::<C>(def)
    }
}

/// Adapter of [`FindMapT<SuperG>`] to [`FindMapT<G>`]
///
/// Aimed for use in super group 'find_map' implementations
pub struct SubGroupFindAdapter<G, SuperG, FindMap>(PhantomData<G>, PhantomData<SuperG>, FindMap);

impl<G, SuperG, FindMap> SubGroupFindAdapter<G, SuperG, FindMap> {
    pub fn new(f: FindMap) -> Self {
        Self(PhantomData::<G>, PhantomData::<SuperG>, f)
    }
}

impl<G, SuperG, FindMap> SubGroupFindAdapter<G, SuperG, FindMap>
where
    G: Group<TopG = SuperG>,
    SuperG: Group<TopG = G::TopG>,
    FindMap: FindMapT<TargetG = SuperG>,
{
    pub fn release_super_map(self) -> FindMap {
        self.2
    }
}

impl<G, SuperG, FindMap> FindMapT for SubGroupFindAdapter<G, SuperG, FindMap>
where
    G: Group<TopG = SuperG>,
    SuperG: Group<TopG = G::TopG>,
    FindMap: FindMapT<TargetG = SuperG>,
{
    type TargetG = G;
    type Outcome = FindMap::Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Result<Self::Outcome, Self>
    where
        C: CurrencyDef + PairsGroup<CommonGroup = <G as Group>::TopG>,
        C::Group: MemberOf<G> + MemberOf<<G as Group>::TopG>,
    {
        self.2.on::<C>(def).map_err(Self::new)
    }
}
