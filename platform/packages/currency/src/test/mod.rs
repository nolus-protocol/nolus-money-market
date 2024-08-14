use std::marker::PhantomData;

use crate::{
    error::Error, AnyVisitor, AnyVisitorPair, AnyVisitorResult, CurrencyDTO, CurrencyDef, Group,
    MemberOf, SingleVisitor,
};

pub use self::group::*;

mod group;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expect<'dto, CDef, VisitedG, VisitorG>(
    &'dto CDef,
    PhantomData<VisitedG>,
    PhantomData<VisitorG>,
);

impl<'dto, CDef, VisitedG, VisitorG> Expect<'dto, CDef, VisitedG, VisitorG> {
    pub fn new(dto: &'dto CDef) -> Self {
        Self(dto, PhantomData, PhantomData)
    }
}
impl<'dto, CDef, VisitedG, VisitorG> AnyVisitor<VisitedG> for Expect<'dto, CDef, VisitedG, VisitorG>
where
    CDef: CurrencyDef,
    CDef::Group: Group + MemberOf<VisitedG>,
    VisitedG: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    type VisitorG = VisitorG;
    type Output = bool;
    type Error = Error;

    fn on<Cin>(self, def: &Cin) -> Result<bool, Error>
    where
        Cin: CurrencyDef,
    {
        Ok(def.dto() == self.0.dto())
    }
}

impl<'dto, CDef, VisitedG, VisitorG> SingleVisitor<CDef>
    for Expect<'dto, CDef, VisitedG, VisitorG>
{
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
    type VisitorG = G;
    type Output = bool;
    type Error = Error;

    fn on<C>(self, _def: &C) -> AnyVisitorResult<G, Self> {
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

pub struct ExpectPair<'dtos, VisitedG1, VisitedG2, G1, G2>(
    &'dtos CurrencyDTO<G1>,
    PhantomData<VisitedG1>,
    &'dtos CurrencyDTO<G2>,
    PhantomData<VisitedG2>,
)
where
    VisitedG1: Group,
    G1: Group + MemberOf<VisitedG1>,
    VisitedG2: Group,
    G2: Group + MemberOf<VisitedG2>;

impl<'dtos, VisitedG1, VisitedG2, G1, G2> ExpectPair<'dtos, VisitedG1, VisitedG2, G1, G2>
where
    VisitedG1: Group,
    G1: Group + MemberOf<VisitedG1>,
    VisitedG2: Group,
    G2: Group + MemberOf<VisitedG2>,
{
    pub fn new(def1: &'dtos CurrencyDTO<G1>, def2: &'dtos CurrencyDTO<G2>) -> Self {
        Self(def1, PhantomData, def2, PhantomData)
    }
}

impl<'dtos, VisitedG1, VisitedG2, G1, G2> AnyVisitorPair
    for ExpectPair<'dtos, VisitedG1, VisitedG2, G1, G2>
where
    VisitedG1: Group,
    G1: Group + MemberOf<VisitedG1>,
    VisitedG2: Group,
    G2: Group + MemberOf<VisitedG2>,
{
    type VisitedG1 = VisitedG1;
    type VisitedG2 = VisitedG2;
    type Output = bool;
    type Error = Error;

    fn on<C1in, C2in>(self, def1: &C1in, def2: &C2in) -> Result<Self::Output, Self::Error>
    where
        C1in: CurrencyDef,
        C2in: CurrencyDef,
    {
        Ok(def1.dto() == self.0 && def2.dto() == self.2)
    }
}
