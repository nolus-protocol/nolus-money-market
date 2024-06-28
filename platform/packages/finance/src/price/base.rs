use currency::{group::MemberOf, Currency, CurrencyDTO, Group};

use crate::{
    coin::{Coin, CoinDTO},
    error::Error,
};

use super::{dto::PriceDTO, Price};

#[derive(Clone, Debug, PartialEq)]
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
    pub fn base_ticker(&self) -> CurrencyDTO<BaseG> {
        self.amount.currency()
    }
}

impl<C, BaseG, QuoteC> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC>
where
    C: Currency + MemberOf<BaseG>,
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
    C: Currency + MemberOf<BaseG>,
    BaseG: Group,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: &BasePrice<BaseG, QuoteC>) -> Result<Self, Self::Error> {
        Ok(super::total_of(value.amount.into()).is(value.amount_quote))
    }
}

impl<BaseG, QuoteC, QuoteG> From<BasePrice<BaseG, QuoteC>> for PriceDTO<BaseG, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(price: BasePrice<BaseG, QuoteC>) -> Self {
        Self::new(price.amount, price.amount_quote.into())
    }
}
