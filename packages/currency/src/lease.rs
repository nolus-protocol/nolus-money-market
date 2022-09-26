use serde::{Deserialize, Serialize};

use finance::currency::{AnyVisitor, Currency, Group, Member, Symbol, SymbolStatic};

use crate::lpn::Usdc;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Atom {}
impl Currency for Atom {
    const SYMBOL: SymbolStatic = "ibc/uh8328hffw";
}
impl Member<LeaseGroup> for Atom {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Osmo {}
impl Currency for Osmo {
    const SYMBOL: SymbolStatic = "ibc/akskvnsf8sfu";
}
impl Member<LeaseGroup> for Osmo {}

// TODO REMOVE once migrate off the single currency version
impl Member<LeaseGroup> for Usdc {}

pub struct LeaseGroup {}

impl Group for LeaseGroup {
    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
    {
        match symbol {
            Atom::SYMBOL => visitor.on::<Atom>(),
            Osmo::SYMBOL => visitor.on::<Osmo>(),
            Usdc::SYMBOL => visitor.on::<Usdc>(),
            _ => visitor.on_unknown(),
        }
    }
}
