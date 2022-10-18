use serde::{Deserialize, Serialize};

use finance::{
    currency::{AnyVisitor, Currency, Group, Member, Symbol, SymbolStatic},
    error::Error,
};

use crate::lpn::Usdc;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Atom {}
impl Currency for Atom {
    const TICKER: SymbolStatic = "ibc/uh8328hffw";
}
impl Member<LeaseGroup> for Atom {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Osmo {}
impl Currency for Osmo {
    const TICKER: SymbolStatic = "ibc/akskvnsf8sfu";
}
impl Member<LeaseGroup> for Osmo {}

// TODO REMOVE once migrate off the single currency version
impl Member<LeaseGroup> for Usdc {}

pub struct LeaseGroup {}

const DESCR: &str = "lease";

impl Group for LeaseGroup {
    type ResolveError = Error;

    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
        Error: Into<V::Error>,
    {
        match symbol {
            Atom::TICKER => visitor.on::<Atom>(),
            Osmo::TICKER => visitor.on::<Osmo>(),
            Usdc::TICKER => visitor.on::<Usdc>(),
            _ => Err(Error::NotInCurrencyGroup(symbol.into(), DESCR.into()).into()),
        }
    }
}
