use currency::{group::MemberOf, Currency, CurrencyDTO, Group};

use crate::coin::{Coin, CoinDTO};

use super::{dto::PriceDTO, Price};

#[derive(Clone, Debug, PartialEq)]
pub struct BasePrice<G, QuoteC>
where
    G: Group,
    QuoteC: ?Sized,
{
    amount: CoinDTO<G>,
    amount_quote: Coin<QuoteC>,
}

impl<G, QuoteC> BasePrice<G, QuoteC>
where
    G: Group,
    QuoteC: Currency,
{
    pub fn base_ticker(&self) -> CurrencyDTO<G> {
        self.amount.currency()
    }
}

impl<C, G, QuoteC> From<Price<C, QuoteC>> for BasePrice<G, QuoteC>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self {
            amount: price.amount.into(),
            amount_quote: price.amount_quote,
        }
    }
}

impl<C, G, QuoteC> From<&BasePrice<G, QuoteC>> for Price<C, QuoteC>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteC: Currency,
{
    fn from(value: &BasePrice<G, QuoteC>) -> Self {
        super::total_of(value.amount.into()).is(value.amount_quote)
    }
}

impl<G, QuoteC, QuoteG> From<BasePrice<G, QuoteC>> for PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(price: BasePrice<G, QuoteC>) -> Self {
        Self::new(price.amount, price.amount_quote.into())
    }
}
