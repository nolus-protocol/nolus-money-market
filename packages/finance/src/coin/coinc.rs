use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    result::Result as StdResult,
};

use sdk::schemars::{self, JsonSchema};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use currency::{
    self, error::CmdError, lpn::Lpns, AnyVisitor, AnyVisitorResult, Currency, Group, SingleVisitor,
    Symbol, SymbolOwned,
};

use crate::{
    coin::Amount,
    error::{Error, Result},
};

use super::{Coin, WithCoin};

pub type LpnCoin = CoinDTO<Lpns>;

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
            type Error = CmdError<V::Error, Error>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency,
            {
                self.1
                    .on::<C>(self.0.amount().into())
                    .map_err(Self::Error::from_customer_err)
            }
        }

        currency::visit_any_on_ticker::<G, _>(&self.ticker, CoinTransformerAny(self, cmd))
            .map_err(CmdError::into_customer_err)
    }
}

impl<G> Display for CoinDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.amount, self.ticker))
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
    fn try_from(coin: &CoinDTO<G>) -> StdResult<Self, Self::Error> {
        struct CoinFactory<'a, G>(&'a CoinDTO<G>);
        impl<'a, G, CC> SingleVisitor<CC> for CoinFactory<'a, G>
        where
            CC: Currency,
        {
            type Output = Coin<CC>;
            type Error = Error;

            fn on(self) -> StdResult<Self::Output, Self::Error> {
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

    fn try_from(coin: CoinDTO<G>) -> StdResult<Self, Self::Error> {
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

pub fn from_amount_ticker<G>(amount: Amount, ticker: Symbol<'_>) -> Result<CoinDTO<G>>
where
    G: Group,
{
    struct Converter<G>(Amount, PhantomData<G>);
    impl<G> AnyVisitor for Converter<G> {
        type Output = CoinDTO<G>;
        type Error = Error;
        fn on<C>(self) -> AnyVisitorResult<Self>
        where
            C: Currency + Serialize + DeserializeOwned,
        {
            Ok(Coin::<C>::from(self.0).into())
        }
    }

    currency::visit_any_on_ticker::<G, _>(ticker, Converter(amount, PhantomData))
}

pub struct IntoDTO<G> {
    _g: PhantomData<G>,
}
impl<G> IntoDTO<G> {
    pub fn new() -> Self {
        Self { _g: PhantomData {} }
    }
}
impl<G> Default for IntoDTO<G> {
    fn default() -> Self {
        Self::new()
    }
}
impl<G> WithCoin for IntoDTO<G> {
    type Output = CoinDTO<G>;
    type Error = Error;

    fn on<C>(&self, coin: Coin<C>) -> super::WithCoinResult<Self>
    where
        C: Currency,
    {
        Ok(coin.into())
    }
}

#[cfg(test)]
mod test {
    use sdk::cosmwasm_std::{from_slice, to_vec};

    use currency::{
        test::{Dai, Nls, TestCurrencies, Usdc},
        Currency, Group, SymbolStatic,
    };

    use crate::{
        coin::{Amount, Coin, CoinDTO},
        error::Error,
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
    fn longer_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        let coin_len = to_vec(&coin).unwrap().len();
        let coindto_len = to_vec(&CoinDTO::<MyTestGroup>::from(coin)).unwrap().len();
        assert!(coin_len < coindto_len);
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

    #[test]
    fn from_amount_ticker_ok() {
        let amount = 20;
        type TheCurrency = Usdc;
        assert_eq!(
            Ok(Coin::<TheCurrency>::from(amount).into()),
            super::from_amount_ticker::<TestCurrencies>(amount, TheCurrency::TICKER)
        );
    }

    #[test]
    fn from_amount_ticker_not_found() {
        let amount = 20;
        type TheCurrency = Usdc;
        assert!(matches!(
            super::from_amount_ticker::<TestCurrencies>(amount, TheCurrency::DEX_SYMBOL),
            Err(Error::CurrencyError { .. })
        ));
        assert!(matches!(
            super::from_amount_ticker::<TestCurrencies>(amount, TheCurrency::BANK_SYMBOL),
            Err(Error::CurrencyError { .. })
        ));
    }

    #[test]
    fn from_amount_ticker_not_in_the_group() {
        assert!(matches!(
            super::from_amount_ticker::<TestCurrencies>(20, Dai::TICKER),
            Err(Error::CurrencyError { .. })
        ));
    }

    #[test]
    fn display() {
        assert_eq!(
            "25 uusdc",
            test_coin::<TestCurrencies, Usdc>(25).to_string()
        );
        assert_eq!("0 unls", test_coin::<TestCurrencies, Nls>(0).to_string());
    }

    fn test_coin<G, C>(amount: Amount) -> CoinDTO<G>
    where
        G: Group,
        C: Currency,
    {
        CoinDTO::<G>::from(Coin::<C>::new(amount))
    }
}
