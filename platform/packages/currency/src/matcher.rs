use std::{any::TypeId, marker::PhantomData};

use crate::{symbol::Symbol, SymbolSlice};

use super::Currency;

pub trait Matcher {
    fn r#match<C>(&self) -> bool
    where
        C: Currency;
}

pub(crate) fn symbol_matcher<'a, S>(symbol: &'a SymbolSlice) -> impl Matcher + 'a
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
    fn r#match<CD>(&self) -> bool
    where
        CD: Currency,
    {
        self.0 == S::symbol::<CD>()
    }
}

#[derive(Debug)]
pub struct TypeMatcher(TypeId);
impl TypeMatcher {
    pub fn new<T>(id: T) -> Self
    where
        T: Into<TypeId>,
    {
        Self(id.into())
    }
}
impl Matcher for TypeMatcher {
    fn r#match<C>(&self) -> bool
    where
        C: 'static,
    {
        TypeId::of::<C>() == self.0
    }
}
