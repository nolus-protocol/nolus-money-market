use crate::{
    coin::{Coin, CoinDTO},
    error::Error,
    price,
};
use currency::{
    self, error::CmdError, AnyVisitor, AnyVisitorResult, Currency, Group, GroupVisit, Tickers,
};

use super::{PriceDTO, WithQuote};

struct BaseCVisitor<'a, G, QuoteC, Cmd>
where
    G: Group,
    QuoteC: Currency,
{
    base_dto: &'a CoinDTO<G>,
    quote: Coin<QuoteC>,
    cmd: Cmd,
}

impl<'a, G, C, Cmd> AnyVisitor for BaseCVisitor<'a, G, C, Cmd>
where
    G: Group,
    C: Currency,
    Cmd: WithQuote<C>,
{
    type Output = Cmd::Output;
    type Error = CmdError<Cmd::Error, Error>;

    #[track_caller]
    fn on<BaseC>(self) -> AnyVisitorResult<Self>
    where
        BaseC: Currency,
    {
        let amount_base =
            Coin::<BaseC>::try_from(self.base_dto).expect("Got different currency in visitor!");
        let price = price::total_of(amount_base).is(self.quote);
        self.cmd.exec(price).map_err(Self::Error::from_customer_err)
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
    //TODO use CoinDTO::with_coin instead
    Tickers
        .visit_any::<G, _>(
            &price.amount.ticker().clone(),
            BaseCVisitor {
                base_dto: &price.amount,
                quote: Coin::<QuoteC>::try_from(&price.amount_quote)
                    .expect("Got different currency in visitor!"),
                cmd,
            },
        )
        .map_err(CmdError::into_customer_err)
}
