use finance::{
    currency::{AnyVisitor, Currency, Group, Member, Symbol},
    error::Error,
};

#[cfg(feature = "testing")]
use crate::test::{TestCurrencyA, TestCurrencyB, TestCurrencyC, TestCurrencyD};
use crate::{
    lease::{Atom, Osmo},
    lpn::Usdc,
    native::Nls,
};

impl Member<PaymentGroup> for Usdc {}
impl Member<PaymentGroup> for Osmo {}
impl Member<PaymentGroup> for Atom {}
impl Member<PaymentGroup> for Nls {}

pub struct PaymentGroup {}

const DESCR: &str = "payment";

impl Group for PaymentGroup {
    type ResolveError = Error;

    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
        Self::ResolveError: Into<V::Error>,
    {
        match symbol {
            Usdc::TICKER => visitor.on::<Usdc>(),
            Osmo::TICKER => visitor.on::<Osmo>(),
            Atom::TICKER => visitor.on::<Atom>(),
            Nls::TICKER => visitor.on::<Nls>(),
            #[cfg(feature = "testing")]
            TestCurrencyA::TICKER => visitor.on::<TestCurrencyA>(),
            #[cfg(feature = "testing")]
            TestCurrencyB::TICKER => visitor.on::<TestCurrencyB>(),
            #[cfg(feature = "testing")]
            TestCurrencyC::TICKER => visitor.on::<TestCurrencyC>(),
            #[cfg(feature = "testing")]
            TestCurrencyD::TICKER => visitor.on::<TestCurrencyD>(),
            _ => Err(Error::NotInCurrencyGroup(symbol.into(), DESCR.into()).into()),
        }
    }
}
