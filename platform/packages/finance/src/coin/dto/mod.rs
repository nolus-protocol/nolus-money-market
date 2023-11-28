use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    result::Result as StdResult,
};

use serde::{Deserialize, Serialize};

use currency::{
    self, error::CmdError, AnyVisitor, AnyVisitorResult, Currency, CurrencyVisit, Group,
    GroupVisit, SingleVisitor, SymbolOwned, SymbolSlice, Tickers,
};
use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::Amount,
    error::{Error, Result},
};

use super::{Coin, WithCoin};

mod unchecked;

/// A type designed to be used in the init, execute and query incoming messages
/// and everywhere the exact currency is unknown at compile time.
///
/// This is a non-currency-parameterized version of finance::coin::Coin<C> that
/// carries also the currency ticker. The aim is to use it everywhere the cosmwasm
/// framework does not support type parameterization or where the currency type
/// is unknown at compile time.
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    try_from = "unchecked::CoinDTO",
    bound(serialize = "", deserialize = "")
)]
pub struct CoinDTO<G>
where
    G: Group,
{
    amount: Amount,
    // TODO either
    // use a reference type, e.g. SymbolStatic, and validate instances on deserialization, or
    // keep a Coin<C> in a Box<Member<G>> replacing all the struct member variables
    ticker: SymbolOwned,
    #[serde(skip)]
    #[schemars(skip)]
    _g: PhantomData<G>,
}

impl<G> Clone for CoinDTO<G>
where
    G: Group,
{
    fn clone(&self) -> Self {
        Self {
            ticker: self.ticker.clone(),
            ..*self
        }
    }
}

impl<G> CoinDTO<G>
where
    G: Group,
{
    fn new_checked(amount: Amount, ticker: SymbolOwned) -> Result<Self> {
        let res = Self::new_raw(amount, ticker);
        res.invariant_held().map(|_| res)
    }

    fn new_unchecked(amount: Amount, ticker: &SymbolSlice) -> Self {
        let res = Self::new_raw(amount, ticker.into());
        debug_assert_eq!(
            Ok(()),
            res.invariant_held(),
            "Conversion of coin with ticker {ticker} to group '{:?}'",
            G::DESCR
        );
        res
    }

    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn ticker(&self) -> &SymbolOwned {
        &self.ticker
    }

    pub fn is_zero(&self) -> bool {
        self.amount == Amount::default()
    }

    pub fn with_coin<V>(&self, cmd: V) -> StdResult<V::Output, V::Error>
    where
        V: WithCoin,
        Error: Into<V::Error>,
    {
        struct CoinTransformerAny<'a, G, V>(&'a CoinDTO<G>, V)
        where
            G: Group;

        impl<'a, G, V> AnyVisitor for CoinTransformerAny<'a, G, V>
        where
            G: Group,
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

        Tickers
            .visit_any::<G, _>(&self.ticker, CoinTransformerAny(self, cmd))
            .map_err(CmdError::into_customer_err)
    }

    fn new_raw(amount: Amount, ticker: SymbolOwned) -> CoinDTO<G> {
        Self {
            amount,
            ticker,
            _g: PhantomData,
        }
    }

    fn invariant_held(&self) -> Result<()> {
        currency::validate::<G>(&self.ticker).map_err(Into::into)
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
    G: Group,
    C: Currency,
{
    type Error = Error;

    // TODO consider adding a compile-time check that a currency belongs to a group
    // one option is to revive the trait Member<Group> that currencies to impl
    // another option is to add an associated trait type to Currency pointing to its direct group
    // the still open quenstion to the both solution is how to express a 'sub-group' relationship
    fn try_from(coin: &CoinDTO<G>) -> StdResult<Self, Self::Error> {
        struct CoinFactory<'a, G>(&'a CoinDTO<G>)
        where
            G: Group;
        impl<'a, G, CC> SingleVisitor<CC> for CoinFactory<'a, G>
        where
            G: Group,
            CC: Currency,
        {
            type Output = Coin<CC>;
            type Error = Error;

            fn on(self) -> StdResult<Self::Output, Self::Error> {
                Ok(Self::Output::new(self.0.amount))
            }
        }
        Tickers
            .maybe_visit(&coin.ticker, CoinFactory(coin))
            .unwrap_or_else(|_| Err(Error::unexpected_ticker::<_, C>(&coin.ticker)))
    }
}

impl<G, C> TryFrom<CoinDTO<G>> for Coin<C>
where
    G: Group,
    C: Currency,
{
    type Error = Error;

    fn try_from(coin: CoinDTO<G>) -> StdResult<Self, Self::Error> {
        Self::try_from(&coin)
    }
}

impl<G, C> From<Coin<C>> for CoinDTO<G>
where
    G: Group,
    C: Currency,
{
    fn from(coin: Coin<C>) -> Self {
        // TODO consider adding a compile-time check that the currency belongs to the group
        Self::new_unchecked(coin.amount, C::TICKER)
    }
}

// TODO remove this back-door
pub fn from_amount_ticker<G>(amount: Amount, ticker: SymbolOwned) -> Result<CoinDTO<G>>
where
    G: Group,
{
    CoinDTO::new_checked(amount, ticker)
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
impl<G> WithCoin for IntoDTO<G>
where
    G: Group,
{
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
    use currency::{
        test::{SubGroupTestC1, SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, SymbolSlice, SymbolStatic,
    };
    use sdk::{
        cosmwasm_std::{from_json, to_json_vec},
        schemars::{self, JsonSchema},
    };

    use crate::{
        coin::{Amount, Coin, CoinDTO},
        error::Error,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, JsonSchema)]
    struct MyTestCurrency;
    impl Currency for MyTestCurrency {
        const TICKER: SymbolStatic = "qwerty";
        const BANK_SYMBOL: SymbolStatic = "ibc/1";
        const DEX_SYMBOL: SymbolStatic = "ibc/2";
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
    struct MyTestGroup {}
    impl Group for MyTestGroup {
        const DESCR: &'static str = "My Test Group";

        fn maybe_visit<M, V>(
            matcher: &M,
            symbol: &SymbolSlice,
            visitor: V,
        ) -> MaybeAnyVisitResult<V>
        where
            M: Matcher + ?Sized,
            V: AnyVisitor,
        {
            assert!(matcher.match_::<MyTestCurrency>(symbol));
            Ok(visitor.on::<MyTestCurrency>())
        }
    }

    #[test]
    fn longer_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        let coin_len = to_json_vec(&coin).unwrap().len();
        let coindto_len = to_json_vec(&CoinDTO::<MyTestGroup>::from(coin))
            .unwrap()
            .len();
        assert!(coin_len < coindto_len);
    }

    #[test]
    fn compatible_deserialization() {
        let coin = Coin::<MyTestCurrency>::new(85);
        assert_eq!(
            coin,
            to_json_vec(&CoinDTO::<MyTestGroup>::from(coin))
                .and_then(from_json)
                .expect("correct raw bytes")
        );
    }

    #[test]
    fn from_amount_ticker_ok() {
        let amount = 20;
        type TheCurrency = SuperGroupTestC1;
        assert_eq!(
            Ok(Coin::<TheCurrency>::from(amount).into()),
            super::from_amount_ticker::<SuperGroup>(amount, TheCurrency::TICKER.into())
        );
    }

    #[test]
    fn from_amount_ticker_not_found() {
        let amount = 20;
        type TheCurrency = SuperGroupTestC1;
        assert!(matches!(
            super::from_amount_ticker::<SuperGroup>(amount, TheCurrency::DEX_SYMBOL.into()),
            Err(Error::CurrencyError { .. })
        ));
        assert!(matches!(
            super::from_amount_ticker::<SuperGroup>(amount, TheCurrency::BANK_SYMBOL.into()),
            Err(Error::CurrencyError { .. })
        ));
    }

    #[test]
    fn from_amount_ticker_not_in_the_group() {
        assert!(matches!(
            super::from_amount_ticker::<SuperGroup>(20, SubGroupTestC1::TICKER.into()),
            Err(Error::CurrencyError { .. })
        ));
    }

    #[test]
    fn display() {
        assert_eq!(
            format!("25 {}", SuperGroupTestC1::TICKER),
            test_coin::<SuperGroup, SuperGroupTestC1>(25).to_string()
        );
        assert_eq!(
            format!("0 {}", SuperGroupTestC2::TICKER),
            test_coin::<SuperGroup, SuperGroupTestC2>(0).to_string()
        );
    }

    fn test_coin<G, C>(amount: Amount) -> CoinDTO<G>
    where
        G: Group,
        C: Currency,
    {
        CoinDTO::<G>::from(Coin::<C>::new(amount))
    }
}
