use std::{any::TypeId, fmt::Debug};

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
    const BANK_SYMBOL: SymbolStatic;
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

pub fn visit_on_bank_symbol<C, V>(bank_symbol: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    V: SingleVisitor<C>,
    C: Currency,
    Error: Into<V::Error>,
{
    maybe_visit_on_bank_symbol(bank_symbol, visitor)
        .unwrap_or_else(|_| Err(Error::unexpected_bank_symbol::<_, C>(bank_symbol).into()))
}

pub type MaybeVisitResult<C, V> =
    Result<Result<<V as SingleVisitor<C>>::Output, <V as SingleVisitor<C>>::Error>, V>;

pub fn maybe_visit_on_ticker<C, V>(ticker: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    C: Currency,
    V: SingleVisitor<C>,
{
    maybe_visit_impl(ticker, C::TICKER, visitor)
}

pub fn maybe_visit_on_bank_symbol<C, V>(bank_symbol: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    V: SingleVisitor<C>,
    C: Currency,
{
    maybe_visit_impl(bank_symbol, C::BANK_SYMBOL, visitor)
}

pub trait Member<G>
where
    G: Group,
{
}

pub type MaybeAnyVisitResult<G, V> =
    Result<Result<<V as AnyVisitor<G>>::Output, <V as AnyVisitor<G>>::Error>, V>;

pub trait Group {
    const DESCR: SymbolStatic;

    fn maybe_visit_on_ticker<V>(symbol: Symbol, visitor: V) -> MaybeAnyVisitResult<Self, V>
    where
        Self: Sized,
        V: AnyVisitor<Self>;

    fn maybe_visit_on_bank_symbol<V>(
        bank_symbol: Symbol,
        visitor: V,
    ) -> MaybeAnyVisitResult<Self, V>
    where
        Self: Sized,
        V: AnyVisitor<Self>;
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

pub fn visit_any<G, V>(ticker: Symbol, visitor: V) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor<G>,
    Error: Into<V::Error>,
{
    G::maybe_visit_on_ticker(ticker, visitor)
        .unwrap_or_else(|_| Err(Error::not_in_currency_group::<_, G>(ticker).into()))
}

pub fn visit_any_on_bank_symbol<G, V>(
    bank_symbol: Symbol,
    visitor: V,
) -> Result<V::Output, V::Error>
where
    G: Group,
    V: AnyVisitor<G>,
    Error: Into<V::Error>,
{
    G::maybe_visit_on_bank_symbol(bank_symbol, visitor)
        .unwrap_or_else(|_| Err(Error::not_in_currency_group::<_, G>(bank_symbol).into()))
}

fn maybe_visit_impl<C, V>(symbol: Symbol, symbol_exp: Symbol, visitor: V) -> MaybeVisitResult<C, V>
where
    V: SingleVisitor<C>,
    C: Currency,
{
    if symbol == symbol_exp {
        Ok(visitor.on())
    } else {
        Err(visitor)
    }
}

#[cfg(test)]
mod test {
    use std::marker::PhantomData;

    use crate::{
        currency::{Currency, SingleVisitor},
        error::Error,
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
        type Error = Error;

        fn on<Cin>(self) -> Result<Self::Output, Self::Error>
        where
            Cin: 'static,
        {
            assert!(super::equal::<C, Cin>());
            Ok(super::equal::<C, Cin>())
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

        assert_eq!(
            Err(Error::not_in_currency_group::<_, TestCurrencies>(
                Nls::BANK_SYMBOL
            )),
            super::visit_any(Nls::BANK_SYMBOL, ExpectUnknownCurrency)
        );
    }

    #[test]
    fn visit_any_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit_any(DENOM, ExpectUnknownCurrency),
            Err(Error::not_in_currency_group::<_, TestCurrencies>(DENOM)),
        );
    }

    #[test]
    fn visit_on_bank_symbol() {
        let v_usdc = Expect::<Usdc>::new();
        assert_eq!(
            super::visit_on_bank_symbol(Usdc::BANK_SYMBOL, v_usdc),
            Ok(true)
        );

        let v_nls = Expect::<Nls>::new();
        assert_eq!(
            super::visit_on_bank_symbol(Nls::BANK_SYMBOL, v_nls),
            Ok(true)
        );
    }

    #[test]
    fn visit_on_bank_symbol_unexpected() {
        const DENOM: &str = "my_fancy_coin";

        assert_eq!(
            super::visit_on_bank_symbol::<Nls, _>(DENOM, ExpectUnknownCurrency),
            Err(Error::unexpected_bank_symbol::<_, Nls>(DENOM,)),
        );
    }
}
