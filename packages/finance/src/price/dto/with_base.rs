use serde::{de::DeserializeOwned, Serialize};

use crate::{
    coin::{Coin, CoinDTO},
    currency::{visit_any, AnyVisitor, Currency},
    price::Price,
};

use super::{PriceDTO, WithBase};

struct QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithBase<C>,
{
    base: Coin<C>,
    quote_dto: CoinDTO,
    cmd: Cmd,
}

impl<C, Cmd> AnyVisitor for QuoteCVisitor<C, Cmd>
where
    C: Currency + Serialize + DeserializeOwned,
    Cmd: WithBase<C>,
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

pub fn execute<Cmd, C>(price: PriceDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithBase<C>,
    C: Currency + Serialize + DeserializeOwned,
{
    visit_any(
        &price.amount_quote.symbol().clone(),
        QuoteCVisitor {
            base: Coin::<C>::try_from(price.amount).expect("Got different currency in visitor!"),
            quote_dto: price.amount_quote,
            cmd,
        },
    )
}
