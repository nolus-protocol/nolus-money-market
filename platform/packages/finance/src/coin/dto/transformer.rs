use std::marker::PhantomData;

use currency::{error::CmdError, AnyVisitor, AnyVisitorResult, Currency, Group, MemberOf};

use crate::{coin::WithCoin, error::Error};

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
    type Error = CmdError<V::Error, Error>;

    fn on<C>(self) -> AnyVisitorResult<VisitedG, Self>
    where
        C: Currency + MemberOf<VisitedG> + MemberOf<Self::VisitorG>,
    {
        self.2
            .on::<C>(self.0.amount().into())
            .map_err(Self::Error::from_customer_err)
    }
}
