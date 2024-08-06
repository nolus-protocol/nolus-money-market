use std::marker::PhantomData;

use crate::{
    error::Error, AnyVisitor, AnyVisitorPair, AnyVisitorResult, Currency, Definition, Group,
    MemberOf, SingleVisitor,
};

pub use self::group::*;

mod group;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expect<C, VisitedG, VisitorG>(
    PhantomData<C>,
    PhantomData<VisitedG>,
    PhantomData<VisitorG>,
);

impl<C, VisitedG, VisitorG> Default for Expect<C, VisitedG, VisitorG> {
    fn default() -> Self {
        Self(PhantomData, PhantomData, PhantomData)
    }
}
impl<C, VisitedG, VisitorG> AnyVisitor<VisitedG> for Expect<C, VisitedG, VisitorG>
where
    C: Currency + MemberOf<VisitedG>,
    VisitedG: Group + MemberOf<VisitorG>,
    VisitorG: Group,
{
    type VisitorG = VisitorG;
    type Output = bool;
    type Error = Error;

    fn on<Cin>(self) -> Result<bool, Error>
    where
        Cin: 'static,
    {
        Ok(crate::equal::<C, Cin>())
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
    type VisitorG = G;
    type Output = bool;
    type Error = Error;

    fn on<C>(self) -> AnyVisitorResult<G, Self> {
        unreachable!();
    }
}

impl<CDef, G> SingleVisitor<CDef> for ExpectUnknownCurrency<G>
where
    CDef: Definition,
{
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        unreachable!();
    }
}

pub struct ExpectPair<CDef1, VisitedG1, CDef2, VisitedG2>(
    PhantomData<CDef1>,
    PhantomData<VisitedG1>,
    PhantomData<CDef2>,
    PhantomData<VisitedG2>,
);
impl<C1, VisitedG1, C2, VisitedG2> AnyVisitorPair for ExpectPair<C1, VisitedG1, C2, VisitedG2>
where
    C1: Currency,
    VisitedG1: Group,
    C2: Currency,
    VisitedG2: Group,
{
    type VisitedG1 = VisitedG1;
    type VisitedG2 = VisitedG2;
    type Output = bool;
    type Error = Error;

    fn on<C1in, C2in>(self) -> Result<Self::Output, Self::Error>
    where
        C1in: Currency,
        C2in: Currency,
    {
        Ok(crate::equal::<C1, C1in>() && crate::equal::<C2, C2in>())
    }
}

impl<C1, VisitedG1, C2, VisitedG2> Default for ExpectPair<C1, VisitedG1, C2, VisitedG2> {
    fn default() -> Self {
        Self(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        )
    }
}
