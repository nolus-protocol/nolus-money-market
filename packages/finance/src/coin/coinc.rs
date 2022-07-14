use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    currency::{Currency, SymbolOwned},
    error::Error,
};

use super::Coin;

/// A type designed to be used in the init, execute and query incoming messages.
/// It is a non-currency-parameterized version of finance::coin::Coin<C> with
/// the same representation on the wire. The aim is to use it everywhere the cosmwasm
/// framework does not support type parameterization.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CoinC {
    amount: u128,
    symbol: SymbolOwned,
}

impl<C> TryFrom<CoinC> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: CoinC) -> Result<Self, Self::Error> {
        if C::SYMBOL == coin.symbol {
            Ok(Self::new(coin.amount))
        } else {
            Err(Error::UnexpectedCurrency(coin.symbol, C::SYMBOL.into()))
        }
    }
}

impl<C> From<Coin<C>> for CoinC
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        Self {
            amount: coin.amount,
            symbol: C::SYMBOL.into(),
        }
    }
}

#[cfg(feature = "testing")]
pub fn funds<C>(amount: u128) -> CoinC
where
    C: Currency,
{
    Coin::<C>::new(amount).into()
}
