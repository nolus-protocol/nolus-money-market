use std::result::Result as StdResult;

use finance::{
    coin::Coin,
    currency::{
        visit_any_on_bank_symbol, visit_on_bank_symbol, AnyVisitor, Currency, Group, SingleVisitor,
    },
    error::Error as FinanceError,
};
use sdk::cosmwasm_std::Coin as CosmWasmCoin;

use crate::error::{Error, Result};

pub(crate) fn from_cosmwasm_impl<C>(coin: CosmWasmCoin) -> Result<Coin<C>>
where
    C: Currency,
{
    visit_on_bank_symbol(&coin.denom, CoinTransformer(&coin))
}

#[cfg(feature = "testing")]
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
    CosmWasmCoin::new(coin.into(), C::BANK_SYMBOL)
}

pub(crate) fn from_cosmwasm_any_impl<G, V>(
    coin: CosmWasmCoin,
    v: V,
) -> StdResult<V::Output, V::Error>
where
    G: Group,
    V: CoinVisitor,
    FinanceError: Into<V::Error>,
{
    visit_any_on_bank_symbol::<G, _>(&coin.denom, CoinTransformerAny(&coin, v))
}

pub trait CoinVisitor {
    type Output;
    type Error;

    fn on<C>(&self, coin: Coin<C>) -> StdResult<Self::Output, Self::Error>
    where
        C: Currency;
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
}

struct CoinTransformerAny<'a, V>(&'a CosmWasmCoin, V);

impl<'a, G, V> AnyVisitor<G> for CoinTransformerAny<'a, V>
where
    G: Group,
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
}

fn from_cosmwasm_internal<C>(coin: &CosmWasmCoin) -> Coin<C>
where
    C: Currency,
{
    debug_assert_eq!(C::BANK_SYMBOL, coin.denom);
    Coin::new(coin.amount.into())
}

#[cfg(test)]
mod test {
    use finance::{
        currency::Currency,
        test::currency::{Nls, Usdc},
    };
    use sdk::cosmwasm_std::Coin as CosmWasmCoin;

    use crate::{coin_legacy::from_cosmwasm_impl, error::Error};

    use super::{to_cosmwasm_impl, Coin};

    #[test]
    fn test_add() {
        let c1 = Coin::<Nls>::new(10);
        let c2 = Coin::<Nls>::new(20);
        let c12 = Coin::<Nls>::new(30);
        assert_eq!(c12, c1 + c2);
    }

    #[test]
    fn from_cosmwasm() {
        let c1 = from_cosmwasm_impl::<Nls>(CosmWasmCoin::new(12, Nls::BANK_SYMBOL));
        assert_eq!(Ok(Coin::<Nls>::new(12)), c1);
    }
    #[test]
    fn from_cosmwasm_unexpected() {
        let c1 = from_cosmwasm_impl::<Nls>(CosmWasmCoin::new(12, Usdc::BANK_SYMBOL));

        assert_eq!(
            c1,
            Err(Error::Finance(
                finance::error::Error::unexpected_bank_symbol::<_, Nls>(Usdc::BANK_SYMBOL,)
            )),
        );

        let c2 = from_cosmwasm_impl::<Usdc>(CosmWasmCoin::new(12, Nls::BANK_SYMBOL));

        assert_eq!(
            c2,
            Err(Error::Finance(
                finance::error::Error::unexpected_bank_symbol::<_, Usdc>(Nls::BANK_SYMBOL,)
            )),
        );
    }

    #[test]
    fn to_cosmwasm() {
        let amount = 326;
        assert_eq!(
            CosmWasmCoin::new(amount, Nls::BANK_SYMBOL),
            to_cosmwasm_impl(Coin::<Nls>::new(amount))
        );
        assert_eq!(
            CosmWasmCoin::new(amount, Usdc::BANK_SYMBOL),
            to_cosmwasm_impl(Coin::<Usdc>::new(amount))
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
