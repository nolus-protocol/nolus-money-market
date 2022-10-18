use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency, Group},
    price::Price,
};

use super::{PriceDTO, WithBase};

struct QuoteCVisitor<C, Cmd>
where
    C: Currency,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, G, Cmd> AnyVisitor<G> for QuoteCVisitor<C, Cmd>
where
    C: Currency,
    G: Group,
    Cmd: WithBase<C>,
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

pub fn execute<G, Cmd, C>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    Cmd: WithBase<C>,
    C: Currency,
    G::ResolveError: Into<Cmd::Error>,
{
    visit_any::<G, _>(
        &price.amount_quote.ticker().clone(),
        QuoteCVisitor {
            base: Coin::<C>::try_from(price.amount).expect("Got different currency in visitor!"),
            quote_dto: price.amount_quote,
            cmd,
        },
    )
}
