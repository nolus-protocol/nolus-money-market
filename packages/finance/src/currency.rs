use serde::{Deserialize, Serialize};

type Symbol<'a> = &'a str;
type SymbolStatic = &'static str;
pub type SymbolOwned = String;

pub trait Currency: 'static + Copy + Ord + Default {
    const SYMBOL: SymbolStatic;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Usdc;
impl Currency for Usdc {
    const SYMBOL: SymbolStatic = "uusdc";
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default, Serialize, Deserialize)]
pub struct Nls;
impl Currency for Nls {
    const SYMBOL: SymbolStatic = "unls";
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

pub trait AnyVisitor {
    type Output;
    type Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency;
    fn on_unknown(self) -> Result<Self::Output, Self::Error>;
}

pub fn visit_any<V>(symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    V: AnyVisitor,
{
    let any_visitor = AnyVisitorImpl(visitor);
    visit::<Nls, _>(symbol, any_visitor)
        .or_else(|any_visitor| visit::<Usdc, _>(symbol, any_visitor))
        .unwrap_or_else(|any_visitor| any_visitor.0.on_unknown())
}

struct AnyVisitorImpl<V>(V);

impl<C, V> SingleVisitor<C> for AnyVisitorImpl<V>
where
    V: AnyVisitor,
    C: Currency,
{
    type Output = Result<<V as AnyVisitor>::Output, <V as AnyVisitor>::Error>;
    type Error = Self;

    fn on(self) -> Result<Self::Output, Self::Error> {
        Ok(self.0.on::<C>())
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(self)
    }
}

#[cfg(test)]
mod test {
    use std::{
        any::{type_name, TypeId},
        marker::PhantomData,
    };

    use crate::currency::{Currency, Nls, SingleVisitor, Usdc};

    use super::AnyVisitor;

    struct Expect<C>(PhantomData<C>);
    impl<C> Expect<C> {
        fn new() -> Self {
            Self(PhantomData)
        }
    }
    impl<C> AnyVisitor for Expect<C>
    where
        C: 'static,
    {
        type Output = bool;
        type Error = ();

        fn on<Cin>(self) -> Result<Self::Output, Self::Error>
        where
            Cin: 'static,
        {
            assert_eq!(
                TypeId::of::<C>(),
                TypeId::of::<Cin>(),
                "Expected {}, got {}",
                type_name::<C>(),
                type_name::<Cin>()
            );
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
    impl AnyVisitor for ExpectUnknownCurrency {
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
