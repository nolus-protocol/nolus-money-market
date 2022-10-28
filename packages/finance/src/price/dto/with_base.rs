use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any_on_ticker, AnyVisitor, Currency, Group},
    error::Error,
    price::Price,
};

use super::{PriceDTO, WithBase};

struct QuoteCVisitor<'a, C, Cmd>
where
    C: Currency,
{
    base: Coin<C>,
    quote_dto: &'a CoinDTO,
    cmd: Cmd,
}

impl<'a, C, Cmd> AnyVisitor for QuoteCVisitor<'a, C, Cmd>
where
    C: Currency,
    Cmd: WithBase<C>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
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

#[track_caller]
pub fn execute<G, Cmd, C>(price: &PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    Cmd: WithBase<C>,
    C: Currency,
    Error: Into<Cmd::Error>,
{
    visit_any_on_ticker::<G, _>(
        &price.amount_quote.ticker().clone(),
        QuoteCVisitor {
            base: Coin::<C>::try_from(&price.amount).expect("Got different currency in visitor!"),
            quote_dto: &price.amount_quote,
            cmd,
        },
    )
}
