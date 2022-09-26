use std::{any::TypeId, fmt::Debug};

use serde::{de::DeserializeOwned, Serialize};

pub type Symbol<'a> = &'a str;
pub type SymbolStatic = &'static str;
pub type SymbolOwned = String;

// Not extending Serialize + DeserializeOwbed since the serde derive implementations fail to
// satisfy trait bounds with regards of the lifetimes
// Foe example, https://stackoverflow.com/questions/70774093/generic-type-that-implements-deserializeowned
pub trait Currency: Copy + Ord + Default + Debug + 'static {
    const SYMBOL: SymbolStatic;
}

pub trait Member<G>
where
    G: Group,
{
}

pub trait Group {
    fn resolve<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
    where
        V: AnyVisitor<Self>,
        Self: Sized;
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
    fn on_unknown(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit<C, V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    V: SingleVisitor<C>,
    C: Currency,
{
    if symbol == C::SYMBOL {
        visitor.on()
    } else {
        visitor.on_unknown()
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
    fn on_unknown(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit_any<G, V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor<G>,
{
    G::resolve(symbol, visitor)
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use crate::{
        currency::{Currency, SingleVisitor},
        test::currency::{Nls, TestCurrencies, Usdc},
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
        type Error = ();

        fn on<Cin>(self) -> Result<Self::Output, Self::Error>
        where
            Cin: 'static,
        {
            assert!(super::equal::<C, Cin>());
            Ok(true)
        }

        fn on_unknown(self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }
    }
    impl<C> SingleVisitor<C> for Expect<C> {
        type Output = bool;
        type Error = ();

        fn on(self) -> Result<Self::Output, Self::Error> {
            Ok(true)
        }

        fn on_unknown(self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }
    }

    struct ExpectUnknownCurrency;
    impl AnyVisitor<TestCurrencies> for ExpectUnknownCurrency {
        type Output = bool;
        type Error = ();

        fn on<C>(self) -> Result<Self::Output, Self::Error>
        where
            C: Currency,
        {
            unreachable!();
        }

        fn on_unknown(self) -> Result<Self::Output, Self::Error> {
            Ok(true)
        }
    }

    impl<C> SingleVisitor<C> for ExpectUnknownCurrency {
        type Output = bool;
        type Error = ();

        fn on(self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }

        fn on_unknown(self) -> Result<Self::Output, Self::Error> {
            Ok(true)
        }
    }
    #[test]
    fn visit_any() {
        let v_usdc = Expect::<Usdc>::new();
        assert_eq!(Ok(true), super::visit_any(Usdc::SYMBOL, v_usdc));

        let v_nls = Expect::<Nls>::new();
        assert_eq!(Ok(true), super::visit_any(Nls::SYMBOL, v_nls));
    }

    #[test]
    fn visit_any_unexpected() {
        assert_eq!(
            Ok(true),
            super::visit_any("my_fancy_coin", ExpectUnknownCurrency)
        );
    }

    #[test]
    fn visit_one() {
        let v_usdc = Expect::<Usdc>::new();
        assert_eq!(Ok(true), super::visit(Usdc::SYMBOL, v_usdc));

        let v_nls = Expect::<Nls>::new();
        assert_eq!(Ok(true), super::visit(Nls::SYMBOL, v_nls));
    }

    #[test]
    fn visit_one_unexpected() {
        assert_eq!(
            Ok(true),
            super::visit::<Nls, _>("my_fancy_coin", ExpectUnknownCurrency)
        );
    }
}
