use std::marker::PhantomData;

use crate::{
    error::Error, AnyVisitor, AnyVisitorPair, AnyVisitorResult, Currency, Group, MemberOf,
    SingleVisitor,
};

pub use self::group::*;

mod group;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expect<C>(PhantomData<C>);

impl<C> Default for Expect<C> {
    fn default() -> Self {
        Self(PhantomData)
    }
}
impl<C, G> AnyVisitor<G> for Expect<C>
where
    C: Currency + MemberOf<G>,
    G: Group,
{
    type VisitorG = G;
    type Output = bool;
    type Error = Error;

    // fn on<Cin>(self) -> AnyVisitorResult<G, Self>
    fn on<Cin>(self) -> Result<bool, Error>
    where
        Cin: 'static,
    {
        Ok(crate::equal::<C, Cin>())
    }
}
impl<C> SingleVisitor<C> for Expect<C> {
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        Ok(true)
    }
}

#[derive(Default)]
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

impl<C, G> SingleVisitor<C> for ExpectUnknownCurrency<G> {
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        unreachable!();
    }
}

pub struct ExpectPair<C1, VisitedG1, C2, VisitedG2>(
    PhantomData<C1>,
    PhantomData<VisitedG1>,
    PhantomData<C2>,
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
        C1in: Currency + MemberOf<Self::VisitedG1>,
        C2in: Currency + MemberOf<Self::VisitedG2>,
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
