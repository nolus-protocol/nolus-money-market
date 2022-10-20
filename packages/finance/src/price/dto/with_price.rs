use std::marker::PhantomData;

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any_on_ticker, AnyVisitor, Currency, Group},
    error::Error,
    price::Price,
};

use super::{PriceDTO, WithPrice};

pub fn execute<G, Cmd>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    Cmd: WithPrice,
    Error: Into<Cmd::Error>,
{
    visit_any_on_ticker::<G, _>(
        &price.amount.ticker().clone(),
        CVisitor {
            group: PhantomData::<G>,
            price_dto: price,
            cmd,
        },
    )
}

struct CVisitor<G, Cmd> {
    group: PhantomData<G>,
    price_dto: PriceDTO,
    cmd: Cmd,
}

impl<G, Cmd> AnyVisitor for CVisitor<G, Cmd>
where
    G: Group,
    Cmd: WithPrice,
    Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
    {
        visit_any_on_ticker::<G, _>(
            &self.price_dto.amount_quote.ticker().clone(),
            QuoteCVisitor {
                base: Coin::<C>::try_from(self.price_dto.amount)
                    .expect("Got different currency in visitor!"),
                quote_dto: self.price_dto.amount_quote,
                cmd: self.cmd,
            },
        )
    }
}

struct QuoteCVisitor<C, Cmd>
where
    C: Currency,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, Cmd> AnyVisitor for QuoteCVisitor<C, Cmd>
where
    C: Currency,
    Cmd: WithPrice,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency,
    {
        self.cmd.exec(Price::new(
            self.base,
            Coin::<QuoteC>::try_from(self.quote_dto).expect("Got different currency in visitor!"),
        ))
    }
}
