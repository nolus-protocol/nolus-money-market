use std::{any::TypeId, marker::PhantomData};

use crate::{symbol::Symbol, Group, MemberOf, SymbolSlice};

use super::Currency;

pub trait Matcher {
    type Group: Group;

    fn r#match<C>(&self) -> bool
    where
        C: Currency + MemberOf<Self::Group>;
}

pub(crate) fn symbol_matcher<'a, S>(symbol: &'a SymbolSlice) -> impl Matcher<Group = S::Group> + 'a
where
    S: 'a + Symbol + ?Sized,
{
    SymbolMatcher::<'a, S>(symbol, PhantomData)
}

struct SymbolMatcher<'a, S>(&'a SymbolSlice, PhantomData<S>)
where
    S: ?Sized;
impl<'a, S> Matcher for SymbolMatcher<'a, S>
where
    S: Symbol + ?Sized,
{
    type Group = S::Group;

    fn r#match<CD>(&self) -> bool
    where
        CD: Currency + MemberOf<Self::Group>,
    {
        self.0 == S::symbol::<CD>()
    }
}

#[derive(Debug)]
pub struct TypeMatcher<G>(TypeId, PhantomData<G>);
impl<G> TypeMatcher<G> {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<TypeId>,
    {
        Self(id.into(), PhantomData)
    }
}
impl<G> Matcher for TypeMatcher<G>
where
    G: Group,
{
    type Group = G;
    fn r#match<C>(&self) -> bool
    where
        C: 'static,
    {
        TypeId::of::<C>() == self.0
    }
}
