use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
    result::Result as StdResult,
};

use serde::{Deserialize, Serialize};

use currency::{
    error::CmdError, group::MemberOf, never::Never, AnyVisitor, AnyVisitorResult, Currency,
    CurrencyDTO, Group,
};
use sdk::schemars::{self, JsonSchema};

use crate::{coin::Amount, error::Error};

use super::{Coin, WithCoin};

/// A type designed to be used in the init, execute and query incoming messages
/// and everywhere the exact currency is unknown at compile time.
///
/// This is a non-currency-type-parameterized version of finance::coin::Coin<C> that
/// carries also the currency. The aim is to use it everywhere the cosmwasm
/// framework does not support type parameterization or where the currency type
/// is unknown at compile time.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(
    deny_unknown_fields,
    rename_all = "snake_case",
    bound(serialize = "", deserialize = "")
)]
pub struct CoinDTO<G>
where
    G: Group,
{
    amount: Amount,
    #[serde(rename="ticker")]
    currency: CurrencyDTO<G>,
}

impl<G> CoinDTO<G>
where
    G: Group,
{
    fn new(amount: Amount, currency: CurrencyDTO<G>) -> Self {
        Self { amount, currency }
    }

    // TODO revisit the need of accesor methods and their potential substitution with `with_coin`
    pub const fn amount(&self) -> Amount {
        self.amount
    }

    pub const fn currency(&self) -> CurrencyDTO<G> {
        self.currency
    }

    pub fn is_zero(&self) -> bool {
        self.amount == Amount::default()
    }

    pub fn with_coin<V>(&self, cmd: V) -> StdResult<V::Output, V::Error>
    where
        V: WithCoin<G>,
        Error: Into<V::Error>,
    {
        struct CoinTransformerAny<'a, G, V>(&'a CoinDTO<G>, V)
        where
            G: Group;

        impl<'a, G, V> AnyVisitor for CoinTransformerAny<'a, G, V>
        where
            G: Group,
            V: WithCoin<G>,
        {
            type VisitedG = G;

            type Output = V::Output;
            type Error = CmdError<V::Error, Error>;

            fn on<C>(self) -> AnyVisitorResult<Self>
            where
                C: Currency + MemberOf<G>,
            {
                self.1
                    .on::<C>(self.0.amount().into())
                    .map_err(Self::Error::from_customer_err)
            }
        }

        self.currency
            .into_currency_type(CoinTransformerAny(self, cmd))
            .map_err(CmdError::into_customer_err)
    }
}

impl<G> Display for CoinDTO<G>
where
    G: Group,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} {}", self.amount, self.currency))
    }
}

impl<G, C> From<&CoinDTO<G>> for Coin<C>
where
    G: Group,
    C: Currency + MemberOf<G>,
{
    fn from(coin: &CoinDTO<G>) -> Self {
        assert_eq!(coin.currency, CurrencyDTO::<G>::from_currency_type::<C>());

        Self::new(coin.amount)
    }
}

impl<G, C> From<CoinDTO<G>> for Coin<C>
where
    G: Group,
    C: Currency + MemberOf<G>,
{
    fn from(coin: CoinDTO<G>) -> Self {
        Self::from(&coin)
    }
}

impl<G, C> From<Coin<C>> for CoinDTO<G>
where
    G: Group,
    C: Currency + ?Sized + MemberOf<G>,
{
    fn from(coin: Coin<C>) -> Self {
        Self::new(coin.amount, CurrencyDTO::<G>::from_currency_type::<C>())
    }
}

// RENAME *_ticker -> *_currency
// TODO remove usages from non-testing code and put behind `#[cfg(...)]
// #[cfg(any(test, feature = "testing"))]
pub fn from_amount_ticker<G>(amount: Amount, currency: CurrencyDTO<G>) -> CoinDTO<G>
where
    G: Group,
{
    CoinDTO::new(amount, currency)
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

impl<G> WithCoin<G> for IntoDTO<G>
where
    G: Group,
{
    type Output = CoinDTO<G>;
    type Error = Never;

    fn on<C>(self, coin: Coin<C>) -> super::WithCoinResult<G, Self>
    where
        C: Currency + MemberOf<G>,
    {
        Ok(coin.into())
    }
}

#[cfg(test)]
mod test {
    use std::panic;

    use serde::{Deserialize, Serialize};

    use currency::{
        group::MemberOf,
        test::{
            SubGroup, SubGroupTestC1, SuperGroup, SuperGroupCurrency, SuperGroupTestC1,
            SuperGroupTestC2,
        },
        AnyVisitor, Currency, Group, Matcher, MaybeAnyVisitResult, SymbolStatic, Symbols,
    };
    use sdk::cosmwasm_std;

    use crate::coin::{Amount, Coin, CoinDTO};

    #[derive(
        Debug, Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize,
    )]
    struct MyTestCurrency;
    impl Currency for MyTestCurrency {
        type Group = MyTestGroup;
    }
    impl Symbols for MyTestCurrency {
        const TICKER: SymbolStatic = "qwerty";

        const BANK_SYMBOL: SymbolStatic = "ibc/1";

        const DEX_SYMBOL: SymbolStatic = "ibc/2";

        const DECIMAL_DIGITS: u8 = 0;
    }

    #[derive(Copy, Clone, Debug, PartialEq)]
    struct MyTestGroup {}

    impl MemberOf<Self> for MyTestGroup {}

    impl Group for MyTestGroup {
        const DESCR: &'static str = "My Test Group";

        fn maybe_visit<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
        where
            M: Matcher + ?Sized,
            V: AnyVisitor<VisitedG = Self>,
        {
            Self::maybe_visit_member(matcher, visitor)
        }

        fn maybe_visit_member<M, V>(matcher: &M, visitor: V) -> MaybeAnyVisitResult<V>
        where
            M: Matcher + ?Sized,
            V: AnyVisitor,
            Self: MemberOf<V::VisitedG>,
        {
            assert!(matcher.match_::<MyTestCurrency>());
            Ok(visitor.on::<MyTestCurrency>())
        }
    }

    #[test]
    fn longer_representation() {
        let coin = Coin::<MyTestCurrency>::new(4215);
        let coin_len = cosmwasm_std::to_json_vec(&coin).unwrap().len();
        let coindto_len = cosmwasm_std::to_json_vec(&CoinDTO::<MyTestGroup>::from(coin))
            .unwrap()
            .len();
        assert!(coin_len < coindto_len);
    }

    #[test]
    fn compatible_deserialization() {
        let coin = Coin::<MyTestCurrency>::new(85);
        assert_eq!(
            coin,
            cosmwasm_std::to_json_vec(&CoinDTO::<MyTestGroup>::from(coin))
                .and_then(cosmwasm_std::from_json)
                .expect("correct raw bytes")
        );
    }

    #[test]
    fn from_amount_ticker_ok() {
        let amount = 20;
        type TheCurrency = SuperGroupTestC1;
        assert_eq!(
            CoinDTO::<SuperGroup>::from(Coin::<TheCurrency>::from(amount)),
            super::from_amount_ticker::<SuperGroup>(
                amount,
                SuperGroupCurrency::from_currency_type::<TheCurrency>()
            )
        );
    }

    #[test]
    fn display() {
        assert_eq!(
            format!("25 {}", SuperGroupTestC1::TICKER),
            test_coin::<SuperGroupTestC1, SuperGroup>(25).to_string()
        );
        assert_eq!(
            format!("0 {}", SuperGroupTestC2::TICKER),
            test_coin::<SuperGroupTestC2, SuperGroup>(0).to_string()
        );
    }

    #[test]
    fn try_from() {
        let test_dto = test_coin::<SuperGroupTestC1, SuperGroup>(123);

        panic::catch_unwind(|| Coin::<SuperGroupTestC2>::from(test_dto))
            .expect_err("Try_into another currency of the same group should fail");
    }

    #[test]
    fn deser_same_group() {
        let coin: CoinDTO<SuperGroup> = Coin::<SuperGroupTestC1>::new(4215).into();
        let coin_deser: CoinDTO<SuperGroup> = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        assert_eq!(coin, coin_deser);
    }

    #[test]
    fn deser_parent_group() {
        type CoinCurrency = SubGroupTestC1;
        type DirectGroup = SubGroup;
        type ParentGroup = SuperGroup;

        let coin: CoinDTO<DirectGroup> = Coin::<CoinCurrency>::new(4215).into();
        let coin_deser: CoinDTO<ParentGroup> = cosmwasm_std::to_json_vec(&coin)
            .and_then(cosmwasm_std::from_json)
            .expect("correct raw bytes");
        let coin_exp: CoinDTO<ParentGroup> = Coin::<CoinCurrency>::from(coin).into();
        assert_eq!(coin_exp, coin_deser);
    }

    #[test]
    fn deser_wrong_group() {
        let coin: CoinDTO<SuperGroup> = Coin::<SuperGroupTestC1>::new(4215).into();
        let coin_raw = cosmwasm_std::to_json_vec(&coin).unwrap();

        assert!(cosmwasm_std::from_json::<CoinDTO<SubGroup>>(&coin_raw).is_err());
    }

    fn test_coin<C, G>(amount: Amount) -> CoinDTO<G>
    where
        C: Currency + MemberOf<G>,
        G: Group,
    {
        CoinDTO::<G>::from(Coin::<C>::new(amount))
    }
}
