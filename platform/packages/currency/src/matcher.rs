use std::{any::TypeId, marker::PhantomData};

use crate::{symbol::Symbol, Definition, Group, MemberOf, SymbolSlice};

pub trait Matcher {
    type Group: Group;

    fn r#match<C>(&self) -> bool
    where
        C: Definition + MemberOf<Self::Group>;

    fn to_sub_matcher<SubG>(&self) -> impl Matcher<Group = SubG>
    where
        SubG: Group + MemberOf<Self::Group>;
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
        CD: Definition,
    {
        self.0 == S::symbol::<CD>()
    }

    fn to_sub_matcher<SubG>(&self) -> impl Matcher<Group = SubG>
    where
        SubG: Group + MemberOf<Self::Group>,
    {
        SymbolMatcher(self.0, PhantomData::<S::Symbol<SubG>>)
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

    fn to_sub_matcher<SubG>(&self) -> impl Matcher<Group = SubG>
    where
        SubG: Group,
    {
        TypeMatcher(self.0, PhantomData)
    }
}
