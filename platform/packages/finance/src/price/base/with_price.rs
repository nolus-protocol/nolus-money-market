use currency::MemberOf;
use currency::{Currency, Group};

use crate::coin::{Coin, WithCoin, WithCoinResult};
use crate::price;
use crate::price::Price;

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
{
    price.amount.with_coin(CoinResolve { price, cmd })
}

struct CoinResolve<'a, G, QuoteC, QuoteG, Cmd>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
{
    price: &'a BasePrice<G, QuoteC, QuoteG>,
    cmd: Cmd,
}

impl<'a, G, QuoteC, QuoteG, Cmd> WithCoin<G> for CoinResolve<'a, G, QuoteC, QuoteG, Cmd>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
{
    type VisitorG = G;

    type Output = Cmd::Output;

    type Error = Cmd::Error;

    fn on<C>(self, amount: Coin<C>) -> WithCoinResult<G, Self>
    where
        C: Currency + MemberOf<Self::VisitorG>,
    {
        self.cmd
            .exec(price::total_of(amount).is(self.price.amount_quote))
    }
}
