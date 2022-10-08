use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    coin::Amount,
    currency::{Currency, SymbolOwned},
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
    symbol: SymbolOwned,
}

impl CoinDTO {
    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn symbol(&self) -> &SymbolOwned {
        &self.symbol
    }
}

impl<C> TryFrom<CoinDTO> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: CoinDTO) -> Result<Self, Self::Error> {
        if C::SYMBOL == coin.symbol {
            Ok(Self::new(coin.amount))
        } else {
            Err(Error::UnexpectedCurrency(coin.symbol, C::SYMBOL.into()))
        }
    }
}

impl<C> From<Coin<C>> for CoinDTO
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

#[cfg(test)]
mod test {
    use cosmwasm_std::to_vec;

    use crate::{
        coin::{Coin, CoinDTO},
        currency::{Currency, SymbolStatic},
    };

    #[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    struct MyTestCurrency;
    impl Currency for MyTestCurrency {
        const SYMBOL: SymbolStatic = "qwerty";
    }

    #[test]
    fn same_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        assert_eq!(to_vec(&coin), to_vec(&CoinDTO::from(coin)));
    }
}
