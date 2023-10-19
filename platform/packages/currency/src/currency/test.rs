use std::marker::PhantomData;

use crate::{error::Error, AnyVisitor, AnyVisitorPair, AnyVisitorResult, Currency, SingleVisitor};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Expect<C>(PhantomData<C>);

impl<C> Default for Expect<C> {
    fn default() -> Self {
        Self(Default::default())
    }
}
impl<C> AnyVisitor for Expect<C>
where
    C: 'static,
{
    type Output = bool;
    type Error = Error;

    fn on<Cin>(self) -> AnyVisitorResult<Self>
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

pub struct ExpectUnknownCurrency;
impl AnyVisitor for ExpectUnknownCurrency {
    type Output = bool;
    type Error = Error;

    fn on<C>(self) -> AnyVisitorResult<Self>
    where
        C: Currency,
    {
        unreachable!();
    }
}

impl<C> SingleVisitor<C> for ExpectUnknownCurrency {
    type Output = bool;
    type Error = Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        unreachable!();
    }
}

pub struct ExpectPair<C1, C2>(PhantomData<C1>, PhantomData<C2>);
impl<C1, C2> Default for ExpectPair<C1, C2> {
    fn default() -> Self {
        Self(Default::default(), Default::default())
    }
}
impl<C1, C2> AnyVisitorPair for ExpectPair<C1, C2>
where
    C1: 'static,
    C2: 'static,
{
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
