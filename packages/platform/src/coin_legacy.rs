use std::result::Result as StdResult;

use cosmwasm_std::Coin as CosmWasmCoin;

use finance::{
    coin::Coin,
    currency::{visit, visit_any, AnyVisitor, Currency, SingleVisitor},
};

use crate::error::{Error, Result};

#[deprecated = "Migrate to using finance::bank::BankAccount"]
pub fn from_cosmwasm<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    from_cosmwasm_impl(coin)
}

pub(crate) fn from_cosmwasm_impl<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    visit(&coin.denom, CoinTransformer(&coin))
}

#[deprecated = "Migrate to using finance::bank::BankAccount"]
pub fn to_cosmwasm<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    to_cosmwasm_impl(coin)
}

pub(crate) fn to_cosmwasm_impl<C>(coin: Coin<C>) -> CosmWasmCoin
where
    C: Currency,
{
    CosmWasmCoin::new(coin.into(), C::SYMBOL)
}

pub trait CoinVisitor {
    type Output;
    type Error;

    fn on<C>(&self, coin: Coin<C>) -> StdResult<Self::Output, Self::Error>
    where
        C: Currency;
    fn on_unknown(&self) -> StdResult<Self::Output, Self::Error>;
}

#[deprecated = "Migrate to using finance::bank::BankAccount"]
pub fn from_cosmwasm_any<V>(coin: CosmWasmCoin, v: V) -> StdResult<V::Output, V::Error>
where
    V: CoinVisitor,
{
    from_cosmwasm_any_impl(coin, v)
}

pub(crate) fn from_cosmwasm_any_impl<V>(coin: CosmWasmCoin, v: V) -> StdResult<V::Output, V::Error>
where
    V: CoinVisitor,
{
    visit_any(&coin.denom, CoinTransformerAny(&coin, v))
}

struct CoinTransformer<'a>(&'a CosmWasmCoin);
impl<'a, C> SingleVisitor<C> for CoinTransformer<'a>
where
    C: Currency,
{
    type Output = Coin<C>;

    type Error = Error;

    fn on(self) -> Result<Self::Output> {
        Ok(from_cosmwasm_internal(self.0))
    }

    fn on_unknown(self) -> Result<Self::Output> {
        Err(Error::UnexpectedCurrency(
            self.0.denom.clone(),
            C::SYMBOL.into(),
        ))
    }
}

struct CoinTransformerAny<'a, V>(&'a CosmWasmCoin, V);
impl<'a, V> AnyVisitor for CoinTransformerAny<'a, V>
where
    V: CoinVisitor,
{
    type Output = V::Output;
    type Error = V::Error;

    fn on<C>(self) -> StdResult<Self::Output, Self::Error>
    where
        C: Currency,
    {
        let coin = Coin::new(self.0.amount.into());
        self.1.on::<C>(coin)
    }

    fn on_unknown(self) -> StdResult<Self::Output, Self::Error> {
        self.1.on_unknown()
    }
}

fn from_cosmwasm_internal<C>(coin: &CosmWasmCoin) -> Coin<C>
where
    C: Currency,
{
    debug_assert_eq!(C::SYMBOL, coin.denom);
    Coin::new(coin.amount.into())
}

#[cfg(test)]
mod test {
    use std::{any::type_name, marker::PhantomData};

    use cosmwasm_std::Coin as CosmWasmCoin;

    use finance::currency::{Currency, Nls, Usdc};

    use crate::{
        coin_legacy::{from_cosmwasm_impl, to_cosmwasm_impl},
        error::Error,
    };

    use super::{Coin, CoinVisitor};

    #[test]
    fn test_add() {
        let c1 = Coin::<Nls>::new(10);
        let c2 = Coin::<Nls>::new(20);
        let c12 = Coin::<Nls>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    struct Expect<C>(PhantomData<C>);

    impl<C> Expect<C> {
        fn new() -> Self {
            Self(PhantomData)
        }
    }

    impl<C> CoinVisitor for Expect<C>
    where
        C: Currency,
    {
        type Output = Coin<C>;
        type Error = ();

        fn on<Cin>(&self, coin: Coin<Cin>) -> Result<Self::Output, Self::Error>
        where
            Cin: Currency,
        {
            assert_eq!(type_name::<C>(), type_name::<Cin>(),);

            // TODO functionality to represent a Coin<X> to Coin<Y>, if X==Y
            Ok(Coin::<C>::new(coin.into()))
        }

        fn on_unknown(&self) -> Result<Self::Output, Self::Error> {
            unreachable!();
        }
    }

    struct ExpectUnknownCurrency;
    impl CoinVisitor for ExpectUnknownCurrency {
        type Output = ();
        type Error = ();

        fn on<Cin>(&self, _coin: Coin<Cin>) -> Result<Self::Output, Self::Error>
        where
            Cin: Currency,
        {
            Err(())
        }

        fn on_unknown(&self) -> Result<Self::Output, Self::Error> {
            Ok(())
        }
    }

    #[test]
    fn from_cosmwasm() {
        let c1 = super::from_cosmwasm_impl::<Nls>(CosmWasmCoin::new(12, Nls::SYMBOL));
        assert_eq!(Ok(Coin::<Nls>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = super::from_cosmwasm_impl::<Nls>(CosmWasmCoin::new(12, Usdc::SYMBOL));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Usdc::SYMBOL.into(),
                Nls::SYMBOL.into()
            )),
            c1
        );
        let c2 = super::from_cosmwasm_impl::<Usdc>(CosmWasmCoin::new(12, Nls::SYMBOL));
        assert_eq!(
            Err(Error::UnexpectedCurrency(
                Nls::SYMBOL.into(),
                Usdc::SYMBOL.into(),
            )),
            c2
        );
    }

    #[test]
    fn from_cosmwasm_any() {
        type T = Nls;
        let v = Expect::<T>::new();
        let amount = 12;
        assert_eq!(
            Ok(Coin::<T>::new(amount)),
            super::from_cosmwasm_any_impl(CosmWasmCoin::new(amount, T::SYMBOL), v)
        );
    }

    #[test]
    #[should_panic]
    fn from_cosmwasm_any_other_currency() {
        let v = Expect::<Usdc>::new();
        let amount = 12;
        let _ = super::from_cosmwasm_any_impl(CosmWasmCoin::new(amount, Nls::SYMBOL), v);
    }

    #[test]
    fn from_cosmwasm_any_unexpected() {
        assert_eq!(
            Ok(()),
            super::from_cosmwasm_any_impl(
                CosmWasmCoin::new(3, "my-nice-currency"),
                ExpectUnknownCurrency
            )
        );
    }

    #[test]
    fn to_cosmwasm() {
        let amount = 326;
        assert_eq!(
            CosmWasmCoin::new(amount, Nls::SYMBOL),
            super::to_cosmwasm_impl(Coin::<Nls>::new(amount))
        );
        assert_eq!(
            CosmWasmCoin::new(amount, Usdc::SYMBOL),
            super::to_cosmwasm_impl(Coin::<Usdc>::new(amount))
        );
    }

    #[test]
    fn from_to_cosmwasm() {
        let c_nls = Coin::<Nls>::new(24563);
        assert_eq!(Ok(c_nls), from_cosmwasm_impl(to_cosmwasm_impl(c_nls)));

        let c_usdc = Coin::<Usdc>::new(u128::MAX);
        assert_eq!(Ok(c_usdc), from_cosmwasm_impl(to_cosmwasm_impl(c_usdc)));
    }
}
