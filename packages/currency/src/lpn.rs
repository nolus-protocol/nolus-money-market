use serde::{Deserialize, Serialize};

use finance::{
    currency::{AnyVisitor, Currency, Group, Member, Symbol, SymbolStatic},
    error::Error,
};

use crate::lease::Atom;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Usdc;
impl Currency for Usdc {
    const SYMBOL: SymbolStatic = "ibc/fj29fj0fj";
}
impl Member<Lpns> for Usdc {}

// TODO REMOVE once migrate off the single currency version
impl Member<Lpns> for Atom {}

pub struct Lpns {}
impl Group for Lpns {
    type ResolveError = Error;

    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
        Self::ResolveError: Into<V::Error>,
    {
        match symbol {
            Usdc::SYMBOL => visitor.on::<Usdc>(),
            Atom::SYMBOL => visitor.on::<Atom>(),
            _ => Err(Error::NotInCurrencyGroup(symbol.into()).into()),
        }
    }
}
