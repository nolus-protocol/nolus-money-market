use serde::{de::DeserializeOwned, Serialize};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency, Group},
    price::Price,
};

use super::{PriceDTO, WithPrice};

pub fn execute<G, Cmd>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    Cmd: WithPrice,
{
    visit_any::<G, _>(
        &price.amount.symbol().clone(),
        CVisitor {
            price_dto: price,
            cmd,
        },
    )
}

struct CVisitor<Cmd>
where
    Cmd: WithPrice,
{
    price_dto: PriceDTO,
    cmd: Cmd,
}

impl<G, Cmd> AnyVisitor<G> for CVisitor<Cmd>
where
    G: Group,
    Cmd: WithPrice,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        visit_any::<G, _>(
            &self.price_dto.amount_quote.symbol().clone(),
            QuoteCVisitor {
                base: Coin::<C>::try_from(self.price_dto.amount)
                    .expect("Got different currency in visitor!"),
                quote_dto: self.price_dto.amount_quote,
                cmd: self.cmd,
            },
        )
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown()
    }
}

struct QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithPrice,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, G, Cmd> AnyVisitor<G> for QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    G: Group,
    Cmd: WithPrice,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency + Serialize + DeserializeOwned,
    {
        self.cmd.exec(Price::new(
            self.base,
            Coin::<QuoteC>::try_from(self.quote_dto).expect("Got different currency in visitor!"),
        ))
    }

    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown()
    }
}
