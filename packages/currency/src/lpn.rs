use serde::{Deserialize, Serialize};

use finance::{
    currency::{AnyVisitor, Currency, Group, Member, Symbol, SymbolStatic},
    error::Error,
};

use crate::lease::Atom;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Usdc;
impl Currency for Usdc {
    const TICKER: SymbolStatic = "ibc/fj29fj0fj";
}
impl Member<Lpns> for Usdc {}

// TODO REMOVE once migrate off the single currency version
impl Member<Lpns> for Atom {}

const DESCR: &str = "lpns";

pub struct Lpns {}
impl Group for Lpns {
    type ResolveError = Error;

    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
        Self::ResolveError: Into<V::Error>,
    {
        match symbol {
            Usdc::TICKER => visitor.on::<Usdc>(),
            Atom::TICKER => visitor.on::<Atom>(),
            _ => Err(Error::NotInCurrencyGroup(symbol.into(), DESCR.into()).into()),
        }
    }
}
