use currency::{error::CmdError, AnyVisitorPair, Currency, Group};

use crate::error::Error;

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
    .map_err(CmdError::into_customer_err)
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
    type Error = CmdError<Cmd::Error, Error>;

    fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
    where
        C1: Currency,
        C2: Currency,
    {
        self.price
            .try_into()
            .map_err(Self::Error::from_api_err)
            .and_then(|price| {
                self.cmd
                    .exec::<C1, C2>(price)
                    .map_err(Self::Error::from_customer_err)
            })
    }
}
