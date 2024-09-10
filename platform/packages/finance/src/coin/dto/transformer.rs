use std::marker::PhantomData;

use currency::{AnyVisitor, AnyVisitorResult, CurrencyDef, Group, MemberOf};

use crate::coin::WithCoin;

use super::CoinDTO;

pub(super) struct CoinTransformerAny<'a, VisitedG, V>(
    &'a CoinDTO<VisitedG>,
    PhantomData<VisitedG>,
    V,
)
where
    VisitedG: Group,
    V: WithCoin<VisitedG>;

impl<'a, VisitedG, V> CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group,
    V: WithCoin<VisitedG>,
{
    pub(super) fn new(dto: &'a CoinDTO<VisitedG>, v: V) -> Self {
        Self(dto, PhantomData::<VisitedG>, v)
    }
}

impl<'a, VisitedG, V> AnyVisitor<VisitedG> for CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group,
    V: WithCoin<VisitedG>,
{
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self, def: &C) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<VisitedG> + MemberOf<VisitedG::TopG>,
    {
        self.2.on::<C>(self.0.as_specific::<C, _>(def.dto()))
    }
}
