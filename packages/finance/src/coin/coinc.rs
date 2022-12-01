use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, result::Result as StdResult};

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
pub struct CoinDTO<G> {
    amount: Amount,
    // TODO either
    // use a reference type, e.g. SymbolStatic, and validate instances on deserialization, or
    // keep a Coin<C> in a Box<Member<G>> replacing all the struct member variables
    ticker: SymbolOwned,
    #[serde(skip)]
    _g: PhantomData<G>,
}

impl<G> CoinDTO<G> {
    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn ticker(&self) -> &SymbolOwned {
        &self.ticker
    }

    pub fn is_zero(&self) -> bool {
        self.amount == Amount::default()
    }
}
impl<G> CoinDTO<G>
where
    G: Group,
{
    pub fn with_coin<V>(&self, cmd: V) -> StdResult<V::Output, V::Error>
    where
        V: WithCoin,
        Error: Into<V::Error>,
    {
        struct CoinTransformerAny<'a, G, V>(&'a CoinDTO<G>, V);

        impl<'a, G, V> AnyVisitor for CoinTransformerAny<'a, G, V>
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

impl<G, C> TryFrom<&CoinDTO<G>> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    // TODO consider adding some compile-time check that a currency belongs to a group
    // one option is to revive the trait Member<Group> that currencies to impl
    // another option is to add an associated trait type to Currency pointing to its direct group
    // the still open quenstion to the both solution is how to express a 'sub-group' relationship
    fn try_from(coin: &CoinDTO<G>) -> Result<Self, Self::Error> {
        struct CoinFactory<'a, G>(&'a CoinDTO<G>);
        impl<'a, G, CC> SingleVisitor<CC> for CoinFactory<'a, G>
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

impl<G, C> TryFrom<CoinDTO<G>> for Coin<C>
where
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: CoinDTO<G>) -> Result<Self, Self::Error> {
        Self::try_from(&coin)
    }
}

impl<G, C> From<Coin<C>> for CoinDTO<G>
where
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        Self {
            amount: coin.amount,
            ticker: C::TICKER.into(),
            _g: PhantomData,
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

    struct MyTestGroup {}

    #[test]
    fn same_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        assert_eq!(to_vec(&coin), to_vec(&CoinDTO::<MyTestGroup>::from(coin)));
    }

    #[test]
    fn compatible_deserialization() {
        let coin = Coin::<MyTestCurrency>::new(85);
        assert_eq!(
            coin,
            to_vec(&CoinDTO::<MyTestGroup>::from(coin))
                .and_then(|buf| from_slice(&buf))
                .expect("correct raw bytes")
        );
    }
}
