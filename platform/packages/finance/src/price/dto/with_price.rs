use currency::{error::CmdError, group::MemberOf, AnyVisitorPair, Currency, Group};

use crate::error::Error;

use super::{PriceDTO, WithPrice};

pub fn execute<G, QuoteG, Cmd>(
    price: &PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Error: Into<Cmd::Error>,
{
    currency::visit_any_on_currencies::<G, QuoteG, _>(
        price.amount.currency(),
        price.amount_quote.currency(),
        PairVisitor { price, cmd },
    )
    .map_err(CmdError::into_customer_err)
}

struct PairVisitor<'a, G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
{
    price: &'a PriceDTO<G, QuoteG>,
    cmd: Cmd,
}

impl<'a, G, QuoteG, Cmd> AnyVisitorPair for PairVisitor<'a, G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Error: Into<Cmd::Error>,
{
    type VisitedG1 = Cmd::G;
    type VisitedG2 = Cmd::QuoteG;
    type Output = Cmd::Output;
    type Error = CmdError<Cmd::Error, Error>;

    fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
    where
        C1: Currency + MemberOf<Self::VisitedG1>,
        C2: Currency + MemberOf<Self::VisitedG2>,
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
