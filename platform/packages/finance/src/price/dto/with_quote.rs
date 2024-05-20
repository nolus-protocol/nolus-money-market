use currency::{Currency, Group};

use crate::{
    coin::{Coin, WithCoin},
    error::Error,
    price,
};

use super::{PriceDTO, WithQuote};

struct BaseCoinVisitor<QuoteC, Cmd>
where
    QuoteC: Currency,
{
    quote: Coin<QuoteC>,
    cmd: Cmd,
}

impl<QuoteC, Cmd> WithCoin for BaseCoinVisitor<QuoteC, Cmd>
where
    QuoteC: Currency,
    Cmd: WithQuote<QuoteC>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    #[track_caller]
    fn on<C>(self, base_amount: Coin<C>) -> crate::coin::WithCoinResult<Self>
    where
        C: Currency,
    {
        self.cmd.exec(price::total_of(base_amount).is(self.quote))
    }
}

#[track_caller]
pub fn execute<G, QuoteC, QuoteG, Cmd>(
    price: &PriceDTO<G, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteC: Currency,
    QuoteG: Group,
    Cmd: WithQuote<QuoteC>,
    Error: Into<Cmd::Error>,
{
    price.amount.with_coin(BaseCoinVisitor {
        quote: Coin::<QuoteC>::try_from(&price.amount_quote)
            .expect("Got different currency in visitor!"),
        cmd,
    })
}
