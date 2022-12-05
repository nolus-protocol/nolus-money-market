use finance::currency::{self, AnyVisitorPair, Currency, Group, Symbol};
use finance::error::Error as FinanceError;
use rmp_serde::decode::Error as DecodeError;
use serde::{de::DeserializeOwned, Serialize};

use crate::{feed::PriceFeed, market_price::PriceFeedBin};

pub trait WithPriceFeed {
    type Output;
    type Error;

    fn exec<C, QuoteC>(self, _feed: PriceFeed<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency + Serialize,
        QuoteC: Currency + Serialize;
}

pub fn execute<G, QuoteG, Cmd>(
    currency_ticker: Symbol,
    quote_currency_ticker: Symbol,
    feed_bin: Option<PriceFeedBin>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    G: Group,
    QuoteG: Group,
    Cmd: WithPriceFeed,
    FinanceError: Into<Cmd::Error>,
    DecodeError: Into<Cmd::Error>,
{
    struct PairVisitor<Cmd>
    where
        Cmd: WithPriceFeed,
    {
        feed_bin: Option<PriceFeedBin>,
        cmd: Cmd,
    }

    impl<Cmd> AnyVisitorPair for PairVisitor<Cmd>
    where
        Cmd: WithPriceFeed,
        DecodeError: Into<Cmd::Error>,
    {
        type Output = Cmd::Output;
        type Error = Cmd::Error;

        fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
        where
            C1: Currency + Serialize + DeserializeOwned,
            C2: Currency + Serialize + DeserializeOwned,
        {
            self.feed_bin
                .map_or_else(
                    || Ok(PriceFeed::<C1, C2>::default()),
                    |bin| rmp_serde::from_slice(&bin).map_err(Into::into),
                )
                .and_then(|feed| self.cmd.exec(feed))
        }
    }

    currency::visit_any_on_tickers::<G, QuoteG, _>(
        currency_ticker,
        quote_currency_ticker,
        PairVisitor { feed_bin, cmd },
    )
}
