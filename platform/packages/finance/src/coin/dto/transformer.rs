use currency::{AnyVisitor, CurrencyDTO, CurrencyDef, Group, MemberOf};

use crate::coin::WithCoin;

use super::CoinDTO;

pub(super) struct CoinTransformerAny<'a, VisitedG, V>(&'a CoinDTO<VisitedG>, V)
where
    VisitedG: Group,
    V: WithCoin<VisitedG>;

impl<'a, VisitedG, V> CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group,
    V: WithCoin<VisitedG>,
{
    pub(super) fn new(dto: &'a CoinDTO<VisitedG>, v: V) -> Self {
        Self(dto, v)
    }
}

impl<VisitedG, V> AnyVisitor<VisitedG> for CoinTransformerAny<'_, VisitedG, V>
where
    VisitedG: Group,
    V: WithCoin<VisitedG>,
{
    type Outcome = V::Outcome;

    fn on<C>(self, def: &CurrencyDTO<C::Group>) -> Self::Outcome
    where
        C: CurrencyDef,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
    {
        self.1.on::<C>(self.0.as_specific::<C, _>(def))
    }
}
