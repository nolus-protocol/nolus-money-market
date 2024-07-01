use currency::{group::MemberOf, AnyVisitorPair, Currency, Group};

use super::{PriceDTO, WithPrice};

pub fn execute<G, QuoteG, Cmd>(
    price: PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
{
    currency::visit_any_on_currencies::<G, QuoteG, _>(
        price.amount.currency(),
        price.amount_quote.currency(),
        PairVisitor { price, cmd },
    )
}

struct PairVisitor<G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
{
    price: PriceDTO<G, QuoteG>,
    cmd: Cmd,
}

impl<G, QuoteG, Cmd> AnyVisitorPair for PairVisitor<G, QuoteG, Cmd>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
{
    type VisitedG1 = Cmd::G;
    type VisitedG2 = Cmd::QuoteG;
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
    where
        C1: Currency + MemberOf<Self::VisitedG1>,
        C2: Currency + MemberOf<Self::VisitedG2>,
    {
        self.cmd.exec::<C1, C2>(self.price.into())
    }
}
