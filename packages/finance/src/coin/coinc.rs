use serde::{Deserialize, Serialize};
use std::result::Result as StdResult;

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::Amount,
    currency::{self, AnyVisitor, Currency, Group, SingleVisitor, SymbolOwned},
    error::Error,
};

use super::{Coin, WithCoin};

/// A type designed to be used in the init, execute and query incoming messages.
/// It is a non-currency-parameterized version of finance::coin::Coin<C> with
/// the same representation on the wire. The aim is to use it everywhere the cosmwasm
/// framework does not support type parameterization.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
pub struct CoinDTO {
    amount: Amount,
    ticker: SymbolOwned,
}

impl CoinDTO {
    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn ticker(&self) -> &SymbolOwned {
        &self.ticker
    }

    pub fn with_coin<G, V>(&self, cmd: V) -> StdResult<V::Output, V::Error>
    where
        G: Group,
        V: WithCoin,
        Error: Into<V::Error>,
    {
        struct CoinTransformerAny<'a, V>(&'a CoinDTO, V);

        impl<'a, V> AnyVisitor for CoinTransformerAny<'a, V>
        where
            V: WithCoin,
        {
            type Output = V::Output;
            type Error = V::Error;

            fn on<C>(self) -> StdResult<Self::Output, Self::Error>
            where
                C: Currency,
            {
                self.1.on::<C>(self.0.amount().into())
            }
        }
        currency::visit_any_on_ticker::<G, _>(&self.ticker, CoinTransformerAny(self, cmd))
    }
}

impl<C> TryFrom<&CoinDTO> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: &CoinDTO) -> Result<Self, Self::Error> {
        struct CoinFactory<'a>(&'a CoinDTO);
        impl<'a, CC> SingleVisitor<CC> for CoinFactory<'a>
        where
            CC: Currency,
        {
            type Output = Coin<CC>;
            type Error = Error;

            fn on(self) -> Result<Self::Output, Self::Error> {
                Ok(Self::Output::new(self.0.amount))
            }
        }
        currency::maybe_visit_on_ticker(&coin.ticker, CoinFactory(coin))
            .unwrap_or_else(|_| Err(Error::unexpected_ticker::<_, C>(&coin.ticker)))
    }
}

impl<C> TryFrom<CoinDTO> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: CoinDTO) -> Result<Self, Self::Error> {
        Self::try_from(&coin)
    }
}

impl<C> From<Coin<C>> for CoinDTO
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        Self {
            amount: coin.amount,
            ticker: C::TICKER.into(),
        }
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::{from_slice, to_vec};

    use crate::{
        coin::{Coin, CoinDTO},
        currency::{Currency, SymbolStatic},
    };

    #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    struct MyTestCurrency;
    impl Currency for MyTestCurrency {
        const TICKER: SymbolStatic = "qwerty";
        const BANK_SYMBOL: SymbolStatic = "ibc/1";
        const DEX_SYMBOL: SymbolStatic = "ibc/2";
    }

    #[test]
    fn same_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        assert_eq!(to_vec(&coin), to_vec(&CoinDTO::from(coin)));
    }

    #[test]
    fn compatible_deserialization() {
        let coin = Coin::<MyTestCurrency>::new(85);
        assert_eq!(
            coin,
            to_vec(&CoinDTO::from(coin))
                .and_then(|buf| from_slice(&buf))
                .expect("correct raw bytes")
        );
    }
}
