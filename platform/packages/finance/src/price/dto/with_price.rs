use std::result::Result as StdResult;

use currency::{AnyVisitorPair, Currency, Group, MemberOf};

use crate::{
    coin::CoinDTO,
    error::{Error, Result},
    price::Price,
};

use super::{PriceDTO, WithPrice};

pub fn execute<G, QuoteG, Cmd>(
    price: PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> StdResult<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Cmd::Error: From<Error>,
{
    currency::visit_any_on_currencies::<G, QuoteG, _>(
        price.amount.currency(),
        price.amount_quote.currency(),
        PairVisitor {
            price: PriceDTOFrom(&price),
            cmd,
        },
    )
}

pub fn execute_with_coins<G, QuoteG, Cmd>(
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<QuoteG>,
    cmd: Cmd,
) -> StdResult<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Cmd::Error: From<Error>,
{
    currency::visit_any_on_currencies::<G, QuoteG, _>(
        amount.currency(),
        amount_quote.currency(),
        PairVisitor {
            price: CoinDTOFrom(amount, amount_quote),
            cmd,
        },
    )
}

trait PriceFactory {
    type G: Group;
    type QuoteG: Group;

    fn try_obtain_price<C, QuoteC>(self) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>;
}

struct PriceDTOFrom<'price, G, QuoteG>(&'price PriceDTO<G, QuoteG>)
where
    G: Group,
    QuoteG: Group;
impl<'price, G, QuoteG> PriceFactory for PriceDTOFrom<'price, G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    type G = G;
    type QuoteG = QuoteG;

    fn try_obtain_price<C, QuoteC>(self) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<G>,
        QuoteC: Currency + MemberOf<QuoteG>,
    {
        Ok(self.0.as_specific())
    }
}

struct CoinDTOFrom<G, QuoteG>(CoinDTO<G>, CoinDTO<QuoteG>)
where
    G: Group,
    QuoteG: Group;

impl<G, QuoteG> PriceFactory for CoinDTOFrom<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    type G = G;

    type QuoteG = QuoteG;

    fn try_obtain_price<C, QuoteC>(self) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>,
    {
        Price::try_new(self.0.as_specific::<C>(), self.1.as_specific::<QuoteC>())
    }
}

struct PairVisitor<Price, G, QuoteG, Cmd>
where
    Price: PriceFactory<G = G, QuoteG = QuoteG>,
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice,
{
    price: Price,
    cmd: Cmd,
}

impl<Price, G, QuoteG, Cmd> AnyVisitorPair for PairVisitor<Price, G, QuoteG, Cmd>
where
    Price: PriceFactory<G = G, QuoteG = QuoteG>,
    G: Group,
    QuoteG: Group,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Cmd::Error: From<Error>,
{
    type VisitedG1 = Cmd::G;
    type VisitedG2 = Cmd::QuoteG;

    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<C1, C2>(self) -> StdResult<Self::Output, Self::Error>
    where
        C1: Currency + MemberOf<Self::VisitedG1>,
        C2: Currency + MemberOf<Self::VisitedG2>,
    {
        self.price
            .try_obtain_price::<C1, C2>()
            .map_err(Into::into)
            .and_then(|price| self.cmd.exec(price))
    }
}
