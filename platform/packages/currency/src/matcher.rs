use std::marker::PhantomData;

use crate::{definition::DefinitionRef, symbol::Symbol};

pub trait Matcher {
    fn r#match(&self, def: DefinitionRef) -> bool;
}

pub(crate) fn symbol_matcher<'a, S>(symbol: &'a str) -> impl Matcher + 'a
where
    S: 'a + Symbol + ?Sized,
{
    SymbolMatcher::<'a, S>(symbol, PhantomData)
}

struct SymbolMatcher<'a, S>(&'a str, PhantomData<S>)
where
    S: ?Sized;
impl<S> Matcher for SymbolMatcher<'_, S>
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
impl Matcher for TypeMatcher {
    fn r#match(&self, def: DefinitionRef) -> bool {
        def == self.0
    }
}
