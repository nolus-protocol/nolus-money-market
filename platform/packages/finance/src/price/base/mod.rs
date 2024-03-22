use std::cmp::Ordering;

use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolSlice};

use crate::{
    coin::{Coin, CoinDTO},
    error::{Error, Result as FinanceResult},
    price::{base::with_price::WithPrice, with_price},
};

use super::{dto::PriceDTO, Price};

mod unchecked;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, JsonSchema)]
#[serde(
    try_from = "unchecked::BasePrice<BaseG, QuoteC>",
    bound(serialize = "")
)]
pub struct BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: ?Sized,
{
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
}

impl<BaseG, QuoteC> BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: ?Sized,
{
    fn new(amount: CoinDTO<BaseG>, amount_quote: Coin<QuoteC>) -> Self {
        let res: Self = Self {
            amount,
            amount_quote,
        };

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

impl<C, BaseG, QuoteC> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new(price.amount.into(), price.amount_quote)
    }
}

impl<C, BaseG, QuoteC> TryFrom<&BasePrice<BaseG, QuoteC>> for Price<C, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: &BasePrice<BaseG, QuoteC>) -> Result<Self, Self::Error> {
        Ok(super::total_of((&value.amount).try_into()?).is(value.amount_quote))
    }
}

impl<BaseG, QuoteC, QuoteG> From<BasePrice<BaseG, QuoteC>> for PriceDTO<BaseG, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency,
    QuoteG: Group,
{
    fn from(price: BasePrice<BaseG, QuoteC>) -> Self {
        Self::new(price.amount, price.amount_quote.into())
    }
}

impl<BaseG, QuoteC> PartialOrd for BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: Currency,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        struct Comparator<'a, BaseG, QuoteC>
        where
            BaseG: Group,
            QuoteC: Currency,
        {
            other: &'a BasePrice<BaseG, QuoteC>,
        }

        impl<'a, BaseG, QuoteC> WithPrice<QuoteC> for Comparator<'a, BaseG, QuoteC>
        where
            BaseG: Group,
            QuoteC: Currency,
        {
            type Output = Option<Ordering>;
            type Error = Error;

            fn exec<C>(self, lhs: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
            {
                Price::<C, QuoteC>::try_from(self.other).map(|rhs| lhs.partial_cmp(&rhs))
            }
        }
        with_price::execute(self, Comparator { other })
            .expect("The currencies of both prices should match")
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
        );
    }

    #[test]
    fn base_zero_json() {
        let json = format!(
            r#"{{"amount": {{"amount": "0", "ticker": "{}"}}, "amount_quote": {{"amount": "3", "ticker": "{}"}}}}"#,
            SuperGroupTestC1::TICKER,
            SuperGroupTestC2::TICKER
        );
        assert_err(
            load::<SuperGroup, SuperGroupTestC2>(&json.into_bytes()),
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
            load::<SuperGroup, SuperGroupTestC2>(&json.into_bytes()),
            "not be zero",
        );
    }

    fn new_invalid<BaseC, QuoteC>(amount: Coin<BaseC>, amount_quote: Coin<QuoteC>)
    where
        BaseC: Currency,
        QuoteC: Currency,
    {
        let _base_price = BasePrice::<SuperGroup, QuoteC>::new(amount.into(), amount_quote);
        #[cfg(not(debug_assertions))]
        {
            _base_price
                .invariant_held()
                .expect("should have returned an error");
        }
    }

    fn load<G, QuoteC>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC>>
    where
        G: Group + for<'a> Deserialize<'a>,
        QuoteC: Currency + for<'a> Deserialize<'a>,
    {
        load_with_group::<G, QuoteC>(json)
    }

    fn load_with_group<G, QuoteC>(json: &[u8]) -> StdResult<BasePrice<G, QuoteC>>
    where
        G: Group + for<'a> Deserialize<'a>,
        QuoteC: Currency + for<'a> Deserialize<'a>,
    {
        from_json::<BasePrice<G, QuoteC>>(json)
    }

    fn assert_err<BaseG, QuoteC>(r: Result<BasePrice<BaseG, QuoteC>, StdError>, msg: &str)
    where
        BaseG: Group,
        QuoteC: Currency,
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
