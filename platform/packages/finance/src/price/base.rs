use std::cmp::Ordering;

use sdk::schemars::{self, JsonSchema};
use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolSlice};

use crate::{
    coin::{Coin, CoinDTO},
    error::Error,
    price::{base::with_price::WithPrice, with_price},
};

use super::{dto::PriceDTO, Price};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq, JsonSchema)]
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
    QuoteC: Currency,
{
    pub fn base_ticker(&self) -> &SymbolSlice {
        self.amount.ticker()
    }

    pub(crate) fn amount(&self) -> &CoinDTO<BaseG> {
        &self.amount
    }

    pub(crate) fn amount_quote(&self) -> Coin<QuoteC> {
        self.amount_quote
    }
}

impl<C, BaseG, QuoteC> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self {
            amount: price.amount.into(),
            amount_quote: price.amount_quote,
        }
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
