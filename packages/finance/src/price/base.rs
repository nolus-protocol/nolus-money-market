use crate::{
    coin::{Coin, CoinDTO},
    currency::{Currency, Group, Symbol},
    error::Error,
};

use super::Price;

#[derive(Clone, Debug, PartialEq)]
pub struct BasePrice<BaseG, QuoteC>
where
    QuoteC: Currency,
    BaseG: Group,
{
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
}

impl<BaseG, QuoteC> BasePrice<BaseG, QuoteC>
where
    QuoteC: Currency,
    BaseG: Group,
{
    pub fn base_ticker(&self) -> Symbol<'_> {
        self.amount.ticker()
    }
}

impl<C, QuoteC, BaseG> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
    BaseG: Group,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self {
            amount: price.amount.into(),
            amount_quote: price.amount_quote,
        }
    }
}

impl<C, QuoteC, BaseG> TryFrom<&BasePrice<BaseG, QuoteC>> for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
    BaseG: Group,
{
    type Error = Error;

    fn try_from(value: &BasePrice<BaseG, QuoteC>) -> Result<Self, Self::Error> {
        Ok(super::total_of((&value.amount).try_into()?).is(value.amount_quote))
    }
}
