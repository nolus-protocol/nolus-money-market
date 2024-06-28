use currency::{group::MemberOf, Currency, Group};

use crate::{
    coin::{Coin, WithCoin, WithCoinResult},
    error::Error,
    price,
};

use super::{PriceDTO, WithQuote};

#[track_caller]
pub fn execute<G, QuoteC, QuoteG, Cmd>(
    price: &PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithQuote<QuoteC, BaseG = G>,
    Error: Into<Cmd::Error>,
{
    price.amount.with_coin(BaseCoinVisitor {
        quote: Coin::<QuoteC>::try_from(&price.amount_quote)
            .expect("Got different currency in visitor!"),
        cmd,
    })
}

struct BaseCoinVisitor<QuoteC, Cmd>
where
    QuoteC: Currency,
{
    quote: Coin<QuoteC>,
    cmd: Cmd,
}

impl<G, QuoteC, Cmd> WithCoin<G> for BaseCoinVisitor<QuoteC, Cmd>
where
    G: Group,
    QuoteC: Currency,
    Cmd: WithQuote<QuoteC, BaseG = G>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
    fn on<C>(self, base_amount: Coin<C>) -> WithCoinResult<G, Self>
    where
        C: Currency + MemberOf<G>,
    {
        self.cmd.exec(price::total_of(base_amount).is(self.quote))
    }
}
