use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::Amount,
    currency::{self, Currency, SingleVisitor, SymbolOwned},
    error::Error,
};

use super::Coin;

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
