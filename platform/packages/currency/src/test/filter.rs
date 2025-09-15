use std::marker::PhantomData;

use crate::{CurrencyDTO, CurrencyDef, Group, MemberOf, SymbolStatic, group::FilterMapT};

#[derive(Clone)]
pub(crate) struct Dto<FilterG>(PhantomData<FilterG>);

impl<FilterG> Default for Dto<FilterG> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<FilterG> FilterMapT for Dto<FilterG>
where
    FilterG: Group,
{
    type VisitedG = FilterG;

    type Outcome = CurrencyDTO<FilterG>;

    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef,
        C::Group: MemberOf<FilterG>,
    {
        Some(def.into_super_group())
    }
}

#[derive(Clone)]
pub(crate) struct FindByTicker<FilterG> {
    ticker1: SymbolStatic,
    ticker2: SymbolStatic,
    _g: PhantomData<FilterG>,
}

impl<FilterG> FindByTicker<FilterG> {
    pub fn new(ticker1: SymbolStatic, ticker2: SymbolStatic) -> Self {
        Self {
            ticker1,
            ticker2,
            _g: PhantomData,
        }
    }
}

impl<FilterG> FilterMapT for FindByTicker<FilterG>
where
    FilterG: Group,
{
    type VisitedG = FilterG;

    type Outcome = CurrencyDTO<FilterG>;

    fn on<C>(&self, def: &CurrencyDTO<C::Group>) -> Option<Self::Outcome>
    where
        C: CurrencyDef,
        C::Group: MemberOf<FilterG>,
    {
        (self.ticker1 == C::ticker() || self.ticker2 == C::ticker())
            .then_some(def.into_super_group())
    }
}
