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
    VisitedG: Group + MemberOf<V::VisitorG>,
    V: WithCoin<VisitedG>;

impl<'a, VisitedG, V> CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group + MemberOf<V::VisitorG>,
    V: WithCoin<VisitedG>,
{
    pub(super) fn new(dto: &'a CoinDTO<VisitedG>, v: V) -> Self {
        Self(dto, PhantomData::<VisitedG>, v)
    }
}

impl<'a, VisitedG, V> AnyVisitor<VisitedG> for CoinTransformerAny<'a, VisitedG, V>
where
    VisitedG: Group + MemberOf<V::VisitorG>,
    V: WithCoin<VisitedG>,
{
    type VisitorG = V::VisitorG;
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self, _def: &C) -> AnyVisitorResult<VisitedG, Self>
    where
        C: CurrencyDef + MemberOf<Self::VisitorG>,
        C::Group: MemberOf<VisitedG>,
    {
        self.2.on::<C>(self.0.as_specific::<C>())
    }
}
