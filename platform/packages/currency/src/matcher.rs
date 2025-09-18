use std::marker::PhantomData;

use crate::{definition::DefinitionRef, symbol::Symbol};

pub trait Matcher {
    fn r#match(&self, def: DefinitionRef) -> bool;
}

pub(crate) fn symbol_matcher<S>(symbol: &str) -> impl Matcher
where
    S: Symbol + ?Sized,
{
    SymbolMatcher::<'_, S>(symbol, PhantomData)
}

pub(crate) fn type_matcher(def: DefinitionRef) -> impl Matcher {
    TypeMatcher(def)
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
struct TypeMatcher(DefinitionRef);
impl Matcher for TypeMatcher {
    fn r#match(&self, def: DefinitionRef) -> bool {
        def == self.0
    }
}
