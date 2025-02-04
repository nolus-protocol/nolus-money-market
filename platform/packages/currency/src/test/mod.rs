use std::marker::PhantomData;

use crate::{
    error::Error, AnyVisitor, AnyVisitorPair, AnyVisitorPairResult, AnyVisitorResult, Currency,
    CurrencyDTO, CurrencyDef, Group, MemberOf, SingleVisitor,
};

pub use self::group::*;

mod group;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expect<CDef, VisitedG, VisitorG>(
    PhantomData<CDef>,
    PhantomData<VisitedG>,
    PhantomData<VisitorG>,
)
where
    CDef: 'static;

impl<CDef, VisitedG, VisitorG> Expect<CDef, VisitedG, VisitorG>
where
    CDef: CurrencyDef,
{
    pub fn new() -> Self {
        Self(PhantomData, PhantomData, PhantomData)
    }
}

impl<CDef, VisitedG, VisitorG> Default for Expect<CDef, VisitedG, VisitorG>
where
    CDef: CurrencyDef,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<CDef, VisitedG, VisitorG> AnyVisitor<VisitedG> for Expect<CDef, VisitedG, VisitorG>
where
    CDef: CurrencyDef,
    CDef::Group: Group + MemberOf<VisitedG>,
    VisitedG: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    type Output = bool;
    type Error = Error;

    fn on<Cin>(self, def: &CurrencyDTO<Cin::Group>) -> Result<bool, Error>
    where
        Cin: CurrencyDef,
    {
        Ok(def == CDef::dto())
    }
}

impl<CDef, VisitedG, VisitorG> SingleVisitor<CDef> for Expect<CDef, VisitedG, VisitorG> {
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        Ok(true)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExpectUnknownCurrency<G> {
    visited_group: PhantomData<G>,
}
impl<G> ExpectUnknownCurrency<G> {
    pub fn new() -> Self {
        Self {
            visited_group: PhantomData,
        }
    }
}

impl<G> AnyVisitor<G> for ExpectUnknownCurrency<G>
where
    G: Group,
{
    type Output = bool;
    type Error = Error;

    fn on<C>(self, _def: &CurrencyDTO<C::Group>) -> AnyVisitorResult<G, Self>
    where
        C: CurrencyDef,
    {
        unreachable!();
    }
}

impl<CDef, G> SingleVisitor<CDef> for ExpectUnknownCurrency<G>
where
    CDef: CurrencyDef,
{
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        unreachable!();
    }
}

pub struct ExpectPair<'dtos, VisitedG, G1, G2>(
    PhantomData<VisitedG>,
    &'dtos CurrencyDTO<G1>,
    &'dtos CurrencyDTO<G2>,
)
where
    VisitedG: Group,
    G1: Group + MemberOf<VisitedG>,
    G2: Group + MemberOf<VisitedG>;

impl<'dtos, VisitedG, G1, G2> ExpectPair<'dtos, VisitedG, G1, G2>
where
    VisitedG: Group,
    G1: Group + MemberOf<VisitedG>,
    G2: Group + MemberOf<VisitedG>,
{
    pub fn new(def1: &'dtos CurrencyDTO<G1>, def2: &'dtos CurrencyDTO<G2>) -> Self {
        Self(PhantomData, def1, def2)
    }
}

impl<VisitedG, G1, G2> AnyVisitorPair for ExpectPair<'_, VisitedG, G1, G2>
where
    VisitedG: Group<TopG = VisitedG>,
    G1: Group + MemberOf<VisitedG>,
    G2: Group + MemberOf<VisitedG>,
{
    type VisitedG = VisitedG;

    type Output = bool;
    type Error = Error;

    fn on<C1in, C2in>(
        self,
        dto1: &CurrencyDTO<Self::VisitedG>,
        dto2: &CurrencyDTO<Self::VisitedG>,
    ) -> AnyVisitorPairResult<Self>
    where
        C1in: Currency,
        C2in: Currency,
    {
        Ok(dto1 == self.1 && dto2 == self.2)
    }
}
