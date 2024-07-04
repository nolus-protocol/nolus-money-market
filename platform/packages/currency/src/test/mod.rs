use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use crate::{
    error::Error, group::MemberOf, AnyVisitor, AnyVisitorPair, AnyVisitorResult, Currency,
    Definition, Group, SingleVisitor,
};

pub use self::group::*;

mod group;

pub struct Expect<C, TopG>(PhantomData<C>, PhantomData<TopG>)
where
    C: ?Sized;

impl<C, TopG> Expect<C, TopG> {
    pub fn new() -> Self {
        Self(PhantomData, PhantomData)
    }
}
impl<C, TopG> Clone for Expect<C, TopG>
where
    C: ?Sized,
{
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}

impl<C, TopG> Debug for Expect<C, TopG>
where
    C: ?Sized,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_tuple("Expect").field(&self.0).finish()
    }
}

impl<C, TopG> Default for Expect<C, TopG>
where
    C: ?Sized,
{
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}

impl<C, TopG> PartialEq for Expect<C, TopG> {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}
impl<C, TopG> Eq for Expect<C, TopG> {}

impl<C, TopG> AnyVisitor for Expect<C, TopG>
where
    C: Currency,
    TopG: Group,
{
    type VisitedG = TopG;

    type Output = bool;
    type Error = Error;

    fn on<Cin>(self) -> AnyVisitorResult<Self>
    where
        Cin: Currency + MemberOf<Self::VisitedG>,
    {
        Ok(crate::equal::<C, Cin>())
    }
}
impl<CDef, TopG> SingleVisitor<CDef> for Expect<CDef, TopG>
where
    CDef: Definition,
{
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        Ok(true)
    }
}

pub struct ExpectUnknownCurrency<TopG>(PhantomData<TopG>);
impl<TopG> Default for ExpectUnknownCurrency<TopG> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<G> AnyVisitor for ExpectUnknownCurrency<G>
where
    G: Group,
{
    type VisitedG = G;
    type Output = bool;
    type Error = Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        unreachable!();
    }
}

impl<CDef, TopG> SingleVisitor<CDef> for ExpectUnknownCurrency<TopG>
where
    CDef: Definition,
{
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
