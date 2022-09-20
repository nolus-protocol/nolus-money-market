use serde::{de::DeserializeOwned, Serialize};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency},
    price::Price,
};

use super::{PriceDTO, WithPrice};

struct QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithPrice,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, Cmd> AnyVisitor for QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
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

struct CVisitor<Cmd>
where
    Cmd: WithPrice,
{
    price_dto: PriceDTO,
    cmd: Cmd,
}

impl<Cmd> AnyVisitor for CVisitor<Cmd>
where
    Cmd: WithPrice,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C>(self) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize + DeserializeOwned,
    {
        visit_any(
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

pub fn execute<Cmd>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithPrice,
{
    visit_any(
        &price.amount.symbol().clone(),
        CVisitor {
            price_dto: price,
            cmd,
        },
    )
}
