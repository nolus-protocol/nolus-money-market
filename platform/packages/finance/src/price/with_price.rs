use currency::{error::CmdError, AnyVisitor, Currency, Group, GroupVisit, Tickers};
use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, price::Price};

use crate::price::base::BasePrice;

pub trait WithPrice<QuoteC>
where
    QuoteC: Currency,
{
    type Output;
    type Error;

    fn exec<C>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency;
}

pub fn execute<BaseG, QuoteC, Cmd>(
    price: &BasePrice<BaseG, QuoteC>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    BaseG: Group,
    QuoteC: Currency,
    Cmd: WithPrice<QuoteC>,
    Error: Into<Cmd::Error>,
{
    Tickers
        .visit_any::<BaseG, _>(price.base_ticker(), CurrencyResolve { price, cmd })
        .map_err(CmdError::into_customer_err)
}

struct CurrencyResolve<'a, G, QuoteC, Cmd>
where
    G: Group,
    QuoteC: Currency,
    Cmd: WithPrice<QuoteC>,
{
    price: &'a BasePrice<G, QuoteC>,
    cmd: Cmd,
}

impl<'a, G, QuoteC, Cmd> AnyVisitor for CurrencyResolve<'a, G, QuoteC, Cmd>
where
    G: Group,
    QuoteC: Currency,
    Cmd: WithPrice<QuoteC>,
    Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = CmdError<Cmd::Error, Error>;

    fn on<C>(self) -> currency::AnyVisitorResult<Self>
    where
        C: Currency + Serialize + DeserializeOwned,
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
