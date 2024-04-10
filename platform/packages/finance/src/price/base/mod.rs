use std::marker::PhantomData;

use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolSlice};

use crate::{
    coin::{Coin, CoinDTO},
    error::{Error, Result as FinanceResult},
};

use super::{dto::PriceDTO, Price};

mod unchecked;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, JsonSchema)]
#[serde(
    try_from = "unchecked::BasePrice<BaseG, QuoteG>",
    bound(serialize = "", deserialize = "")
)]
pub struct BasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    amount: CoinDTO<BaseG>,
    #[serde(serialize_with = "serialize_amount_quote::<_, _, QuoteG>")]
    amount_quote: Coin<QuoteC>,
    #[serde(skip)]
    quote_group: PhantomData<QuoteG>,
}

impl<BaseG, QuoteC, QuoteG> BasePrice<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    fn new_checked(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> FinanceResult<Self> {
        let res = Self::new_raw(amount, amount_quote);
        res.invariant_held().map(|_| res)
    }

    fn new_unchecked(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        let res = Self::new_raw(amount, amount_quote);

        debug_assert_eq!(Ok(()), res.invariant_held());
        res
    }

    pub fn base_ticker(&self) -> &SymbolSlice {
        self.amount.ticker()
    }

    pub(crate) fn amount(&self) -> &CoinDTO<BaseG> {
        &self.amount
    }

    pub(crate) fn amount_quote(&self) -> Coin<QuoteC> {
        self.amount_quote
    }

    fn new_raw(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        Self {
            amount,
            amount_quote,
            quote_group: PhantomData,
        }
    }

    fn invariant_held(&self) -> FinanceResult<()> {
        Self::check(!self.amount.is_zero(), "The amount should not be zero").and_then(|_| {
            Self::check(
                !self.amount_quote.is_zero(),
                "The quote amount should not be zero",
            )
        })
    }

    fn check(invariant: bool, msg: &str) -> FinanceResult<()> {
        Error::broken_invariant_if::<Self>(!invariant, msg)
    }
}

fn serialize_amount_quote<S, QuoteC, QuoteG>(
    amount: &Coin<QuoteC>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    QuoteC: Currency + ?Sized,
    QuoteG: Group,
{
    currency::validate_member::<QuoteC, QuoteG>()
        .map_err(|err| {
            serde::ser::Error::custom(format!("Amount quote serializaion failed: {:?}", err))
        })
        .and_then(|_| {
            let coin_dto = CoinDTO::<QuoteG>::from(*amount);
            coin_dto.serialize(serializer)
        })
}

impl<C, BaseG, QuoteC, QuoteG> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC, QuoteG>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new_unchecked(price.amount.into(), price.amount_quote)
    }
}

impl<C, BaseG, QuoteC, QuoteG> TryFrom<&BasePrice<BaseG, QuoteC, QuoteG>> for Price<C, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    type Error = Error;

    fn try_from(value: &BasePrice<BaseG, QuoteC, QuoteG>) -> Result<Self, Self::Error> {
        (&value.amount)
            .try_into()
            .map(|amount| super::total_of(amount).is(value.amount_quote))
            .map_err(Into::into)
    }
}

impl<BaseG, QuoteC, QuoteG> From<BasePrice<BaseG, QuoteC, QuoteG>> for PriceDTO<BaseG, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    fn from(price: BasePrice<BaseG, QuoteC, QuoteG>) -> Self {
        Self::new(price.amount, price.amount_quote.into())
    }
}

#[cfg(test)]
mod test_invariant {
    use currency::{
        test::{SuperGroup, SuperGroupTestC1, SuperGroupTestC2},
        Currency, Group,
    };
    use sdk::cosmwasm_std::{from_json, StdError, StdResult};
    use serde::Deserialize;

    use crate::coin::Coin;

    use super::BasePrice;

    #[test]
    #[should_panic = "zero"]
    fn base_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(0),
            Coin::<SuperGroupTestC2>::new(3),
        )
    }

    #[test]
    fn base_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "0", "ticker": "{}"}}, "amount_quote": {{"amount": "3", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_err(
            load::<SuperGroup, SuperGroupTestC2, SuperGroup>(&json.into_bytes()),
            "not be zero",
        );
    }

    #[test]
    #[should_panic = "zero"]
    fn quote_zero() {
        new_invalid(
            Coin::<SuperGroupTestC1>::new(6),
            Coin::<SuperGroupTestC2>::new(0),
        );
    }

    #[test]
    fn quote_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "6", "ticker": "{}"}}, "amount_quote": {{"amount": "0", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_err(
            load::<SuperGroup, SuperGroupTestC2, SuperGroup>(&json.into_bytes()),
            "not be zero",
        );
    }

    fn new_invalid<BaseC, QuoteC>(amount: Coin<BaseC>, amount_quote: Coin<QuoteC>)
    where
        BaseC: Currency,
        QuoteC: Currency,
    {
        let _base_price: BasePrice<SuperGroup, QuoteC, SuperGroup> =
            BasePrice::new_unchecked(amount.into(), amount_quote);

        #[cfg(not(debug_assertions))]
        {
            _base_price
                .invariant_held()
                .expect("should have returned an error");
        }
    }

    fn load<G, QuoteC, QuoteG>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC, QuoteG>>
    where
        G: Group + for<'a> Deserialize<'a>,
        QuoteC: Currency + for<'a> Deserialize<'a>,
        QuoteG: Group + for<'a> Deserialize<'a>,
    {
        load_with_group::<G, QuoteC, QuoteG>(json)
    }

    fn load_with_group<G, QuoteC, QuoteG>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC, QuoteG>>
    where
        G: Group + for<'a> Deserialize<'a>,
        QuoteC: Currency + for<'a> Deserialize<'a>,
        QuoteG: Group + for<'a> Deserialize<'a>,
    {
        from_json::<BasePrice<G, QuoteC, QuoteG>>(json)
    }

    fn assert_err<BaseG, QuoteC, QuoteG>(r: StdResult<BasePrice<BaseG, QuoteC, QuoteG>>, msg: &str)
    where
        BaseG: Group,
        QuoteC: Currency,
        QuoteG: Group,
    {
        assert!(matches!(
            r,
            Err(StdError::ParseErr {
                target_type,
                msg: real_msg
            }) if target_type.contains("BasePrice") && real_msg.contains(msg)
        ));
    }
}
