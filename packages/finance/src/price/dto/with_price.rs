use serde::{de::DeserializeOwned, Serialize};

use crate::{
    currency::{self, AnyVisitorPair, Currency, Group},
    error::Error,
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
    currency::visit_any_on_tickers::<G, QuoteG, _>(
        price.amount.ticker(),
        price.amount_quote.ticker(),
        PairVisitor { price, cmd },
    )
}

struct PairVisitor<'a, G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice,
{
    price: &'a PriceDTO<G, QuoteG>,
    cmd: Cmd,
}

impl<'a, G, QuoteG, Cmd> AnyVisitorPair for PairVisitor<'a, G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice,
    Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
    where
        C1: Currency + Serialize + DeserializeOwned,
        C2: Currency + Serialize + DeserializeOwned,
    {
        let price = self.price.try_into().map_err(Error::into)?;
        self.cmd.exec::<C1, C2>(price)
    }
}
