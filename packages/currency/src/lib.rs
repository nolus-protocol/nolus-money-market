use std::marker::PhantomData;

use finance::currency::{AnyVisitor, Currency, Group, Member, SingleVisitor};
use serde::{de::DeserializeOwned, Serialize};

pub mod lease;
pub mod lpn;
pub mod native;
pub mod payment;

struct SingleVisitorAdapter<G, V>(V, PhantomData<G>);

impl<G, V> From<V> for SingleVisitorAdapter<G, V> {
    fn from(any_visitor: V) -> Self {
        Self(any_visitor, PhantomData)
    }
}

impl<C, G, V> SingleVisitor<C> for SingleVisitorAdapter<G, V>
where
    C: 'static + Currency + Member<G> + Serialize + DeserializeOwned,
    G: Group,
    V: AnyVisitor<G>,
{
    type Output = V::Output;
    type Error = V::Error;

    fn on(self) -> Result<Self::Output, Self::Error> {
        self.0.on::<C>()
    }
}
