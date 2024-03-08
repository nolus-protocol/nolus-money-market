use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolSlice};

use crate::{
    coin::{Coin, CoinDTO},
    error::Error,
    price::dto::WithPrice,
};

use super::Price;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Eq)]
pub struct BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: ?Sized,
{
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
}

impl<BaseG, QuoteC> BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: Currency,
{
    pub fn base_ticker(&self) -> &SymbolSlice {
        self.amount.ticker()
    }
}

impl<C, BaseG, QuoteC> From<Price<C, QuoteC>> for BasePrice<BaseG, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self {
            amount: price.amount.into(),
            amount_quote: price.amount_quote,
        }
    }
}

impl<C, BaseG, QuoteC> TryFrom<&BasePrice<BaseG, QuoteC>> for Price<C, QuoteC>
where
    C: Currency,
    BaseG: Group,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: &BasePrice<BaseG, QuoteC>) -> Result<Self, Self::Error> {
        Ok(super::total_of((&value.amount).try_into()?).is(value.amount_quote))
    }
}

// impl<C, BaseG, QuoteC> From<&BasePrice<BaseG, QuoteC>> for Price<C, QuoteC>
// where
//     C: Currency,
//     BaseG: Group,
//     QuoteC: Currency,
// {
//     fn from(base_price: &BasePrice<BaseG, QuoteC>) -> Self {
//         Self {
//             amount: base_price.amount.into(),
//             amount_quote: base_price.amount_quote,
//         }
//     }
// }

impl<BaseG, QuoteC> PartialOrd for BasePrice<BaseG, QuoteC>
where
    BaseG: Group,
    QuoteC: Currency,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        struct Comparator<'a, BaseG, QuoteCurrency>
        where
            BaseG: Group,
            QuoteCurrency: Currency,
        {
            other: &'a BasePrice<BaseG, QuoteCurrency>,
        }

        impl<'a, BaseG, QuoteCurrency> WithPrice for Comparator<'a, BaseG, QuoteCurrency>
        where
            BaseG: Group,
            QuoteCurrency: Currency,
        {
            type Output = Option<Ordering>;
            type Error = Error;

            fn exec<C, QuoteC>(self, lhs: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
            where
                C: Currency,
                QuoteC: Currency,
            {
                Price::<C, QuoteC>::try_from(self.other).map(|rhs| lhs.partial_cmp(&rhs))
            }
        }
        with_price::execute(self, Comparator { other })
            .expect("The currencies of both prices should match")
    }
}

mod with_price {
    use currency::{error::CmdError, AnyVisitorPair, Currency, Group, GroupVisit, Tickers};
    use serde::{de::DeserializeOwned, Serialize};

    use crate::{error::Error, price::dto::WithPrice};

    use super::BasePrice;

    pub fn execute<BaseG, QuoteC, Cmd>(
        price: &BasePrice<BaseG, QuoteC>,
        cmd: Cmd,
    ) -> Result<Cmd::Output, Cmd::Error>
    where
        BaseG: Group,
        QuoteC: Currency,
        Cmd: WithPrice,
        Error: Into<Cmd::Error>,
    {
        Tickers
            .visit_any::<BaseG, _>(price.amount.ticker(), PairVisitor { price, cmd })
            .map_err(CmdError::into_customer_err)
    }

    struct PairVisitor<'a, G, QuoteC, Cmd>
    where
        G: Group,
        QuoteC: Currency,
        Cmd: WithPrice,
    {
        price: &'a BasePrice<G, QuoteC>,
        cmd: Cmd,
    }

    impl<'a, G, QuoteC, Cmd> AnyVisitorPair for PairVisitor<'a, G, QuoteC, Cmd>
    where
        G: Group,
        QuoteC: Currency,
        Cmd: WithPrice,
        Error: Into<Cmd::Error>,
    {
        type Output = Cmd::Output;
        type Error = CmdError<Cmd::Error, Error>;

        fn on<C1, C2>(self) -> Result<Self::Output, Self::Error>
        where
            C1: Currency + Serialize + DeserializeOwned,
            C2: Currency + Serialize + DeserializeOwned,
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
}
