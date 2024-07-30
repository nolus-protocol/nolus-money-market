use currency::MemberOf;
use currency::{error::CmdError, AnyVisitor, Currency, Group, GroupVisit, Tickers};

use crate::{error::Error, price::Price};

use crate::price::base::BasePrice;

pub trait WithPrice<QuoteC>
where
    QuoteC: Currency,
{
    type PriceG: Group;

    type Output;
    type Error;

    fn exec<C>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency + MemberOf<Self::PriceG>;
}

pub fn execute<BaseG, QuoteC, QuoteG, Cmd>(
    price: &BasePrice<BaseG, QuoteC, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = BaseG>,
    Error: Into<Cmd::Error>,
{
    Tickers::<BaseG>::visit_any(price.base_ticker(), CurrencyResolve { price, cmd })
        .map_err(CmdError::into_customer_err)
}

struct CurrencyResolve<'a, G, QuoteC, QuoteG, Cmd>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
{
    price: &'a BasePrice<G, QuoteC, QuoteG>,
    cmd: Cmd,
}

impl<'a, G, QuoteC, QuoteG, Cmd> AnyVisitor<G> for CurrencyResolve<'a, G, QuoteC, QuoteG, Cmd>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
    Error: Into<Cmd::Error>,
{
    type VisitorG = G;
    type Output = Cmd::Output;
    type Error = CmdError<Cmd::Error, Error>;

    fn on<C>(self) -> currency::AnyVisitorResult<G, Self>
    where
        C: Currency + MemberOf<Self::VisitorG>,
    {
        self.price
            .try_into()
            .map_err(Self::Error::from_api_err)
            .and_then(|price| {
                self.cmd
                    .exec::<C>(price)
                    .map_err(Self::Error::from_customer_err)
            })
    }
}
