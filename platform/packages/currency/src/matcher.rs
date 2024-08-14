use std::marker::PhantomData;

use crate::{definition::DefinitionRef, symbol::Symbol, SymbolSlice};

pub trait Matcher {
    fn r#match(&self, def: DefinitionRef) -> bool;
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
    fn r#match(&self, def: DefinitionRef) -> bool {
        self.0 == S::symbol(def)
    }
}

#[derive(Debug)]
pub struct TypeMatcher(DefinitionRef);
impl TypeMatcher {
    pub fn new(def: DefinitionRef) -> Self {
        Self(def)
    }
}
impl Matcher for TypeMatcher
{
    fn r#match(&self, def: DefinitionRef) -> bool {
        def == self.0
    }
}
