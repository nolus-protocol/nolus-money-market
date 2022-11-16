use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any_on_ticker, AnyVisitor, Currency, Group},
    error::Error,
    price::{self},
};

use super::{PriceDTO, WithPrice};

pub fn execute<G, QuoteG, Cmd>(
    price: &PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice,
    Error: Into<Cmd::Error>,
{
    visit_any_on_ticker::<G, _>(
        &price.amount.ticker().clone(),
        CVisitor {
            price_dto: price,
            cmd,
        },
    )
}

struct CVisitor<'a, G, QuoteG, Cmd> {
    price_dto: &'a PriceDTO<G, QuoteG>,
    cmd: Cmd,
}

impl<'a, G, QuoteG, Cmd> AnyVisitor for CVisitor<'a, G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice,
    Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        visit_any_on_ticker::<QuoteG, _>(
            &self.price_dto.amount_quote.ticker().clone(),
            QuoteCVisitor {
                base: Coin::<C>::try_from(&self.price_dto.amount)
                    .expect("Got different currency in visitor!"),
                quote_dto: &self.price_dto.amount_quote,
                cmd: self.cmd,
            },
        )
    }
}

struct QuoteCVisitor<'a, QuoteG, C, Cmd>
where
    C: Currency,
{
    base: Coin<C>,
    quote_dto: &'a CoinDTO<QuoteG>,
    cmd: Cmd,
}

impl<'a, QuoteG, C, Cmd> AnyVisitor for QuoteCVisitor<'a, QuoteG, C, Cmd>
where
    C: Currency,
    Cmd: WithPrice,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency,
    {
        let amount_quote =
            Coin::<QuoteC>::try_from(self.quote_dto).expect("Got different currency in visitor!");
        let price = price::total_of(self.base).is(amount_quote);
        self.cmd.exec(price)
    }
}
