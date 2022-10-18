use std::{any::TypeId, error::Error as ErrorTrait, fmt::Debug};

use serde::{de::DeserializeOwned, Serialize};

use crate::error::Error;

pub type Symbol<'a> = &'a str;
pub type SymbolStatic = &'static str;
pub type SymbolOwned = String;

// Not extending Serialize + DeserializeOwbed since the serde derive implementations fail to
// satisfy trait bounds with regards of the lifetimes
// Foe example, https://stackoverflow.com/questions/70774093/generic-type-that-implements-deserializeowned
pub trait Currency: Copy + Ord + Default + Debug + 'static {
    const TICKER: SymbolStatic;
}

pub trait Member<G>
where
    G: Group,
{
}

pub trait Group {
    type ResolveError: ErrorTrait;

    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        Self: Sized,
        V: AnyVisitor<Self>,
        Self::ResolveError: Into<V::Error>;
}

pub fn equal<C1, C2>() -> bool
where
    C1: 'static,
    C2: 'static,
{
    TypeId::of::<C1>() == TypeId::of::<C2>()
}

pub trait SingleVisitor<C> {
    type Output;
    type Error;

    fn on(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit<C, V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    V: SingleVisitor<C>,
    C: Currency,
    Error: Into<V::Error>,
{
    if symbol == C::TICKER {
        visitor.on()
    } else {
        Err(Error::UnexpectedCurrency(symbol.into(), C::TICKER.into()).into())
    }
}

pub trait AnyVisitor<G>
where
    G: Group,
{
    type Output;
    type Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + Member<G> + Serialize + DeserializeOwned;
}

pub fn visit_any<G, V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor<G>,
    G::ResolveError: Into<V::Error>,
{
    G::resolve(symbol, visitor)
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use crate::{
        currency::{Currency, SingleVisitor},
        error::Error,
        test::currency::{Nls, TestCurrencies, Usdc, DESCR},
    };

    use super::AnyVisitor;

    struct Expect<C>(PhantomData<C>);

    impl<C> Expect<C> {
        fn new() -> Self {
            Self(PhantomData)
        }
    }
    impl<C> AnyVisitor<TestCurrencies> for Expect<C>
    where
        C: 'static,
    {
        type Output = bool;
        type Error = Error;

        fn on<Cin>(self) -> Result<Self::Output, Self::Error>
        where
            Cin: 'static,
        {
            assert!(super::equal::<C, Cin>());
            Ok(true)
        }
    }
    impl<C> SingleVisitor<C> for Expect<C> {
        type Output = bool;
        type Error = Error;

        fn on(self) -> Result<Self::Output, Self::Error> {
            Ok(true)
        }
    }

    struct ExpectUnknownCurrency;
    impl AnyVisitor<TestCurrencies> for ExpectUnknownCurrency {
        type Output = bool;
        type Error = Error;

        fn on<C>(self) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
        {
            unreachable!();
        }
    }

    impl<C> SingleVisitor<C> for ExpectUnknownCurrency {
        type Output = bool;
        type Error = Error;

        fn on(self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }
    }
    #[test]
    fn visit_any() {
        let v_usdc = Expect::<Usdc>::new();
        assert_eq!(Ok(true), super::visit_any(Usdc::TICKER, v_usdc));

        let v_nls = Expect::<Nls>::new();
        assert_eq!(Ok(true), super::visit_any(Nls::TICKER, v_nls));
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit_any(DENOM, ExpectUnknownCurrency),
            Err(Error::NotInCurrencyGroup(DENOM.into(), DESCR.into())),
        );
    }

    #[test]
    fn visit_one() {
        let v_usdc = Expect::<Usdc>::new();
        assert_eq!(super::visit(Usdc::TICKER, v_usdc), Ok(true));

        let v_nls = Expect::<Nls>::new();
        assert_eq!(super::visit(Nls::TICKER, v_nls), Ok(true));
    }

    #[test]
    fn visit_one_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit::<Nls, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::UnexpectedCurrency(DENOM.into(), Nls::TICKER.into())),
        );
    }
}
