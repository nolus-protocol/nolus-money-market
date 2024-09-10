use std::marker::PhantomData;

use currency::{Currency, Group};
use currency::{CurrencyDef, MemberOf};

use crate::coin::{Coin, CoinDTO, WithCoin, WithCoinResult};
use crate::error::Error;
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
        C: CurrencyDef,
        C::Group: MemberOf<Self::PriceG>;
}

/// Execute the provided price command on a valid base price
pub fn execute<BaseG, QuoteC, QuoteG, Cmd>(
    price: &BasePrice<BaseG, QuoteC, QuoteG>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    BaseG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = BaseG>,
    Cmd::Error: From<Error>,
{
    price.amount.with_super_coin(CoinResolve {
        price: UncheckedConversion(price),
        cmd,
    })
}

/// Execute the provided price command on a non-validated price
/// Intended mainly for invariant validation purposes.
pub(super) fn execute_with_coins<BaseG, QuoteC, QuoteG, Cmd>(
    amount: CoinDTO<BaseG>,
    amount_quote: Coin<QuoteC>,
    cmd: Cmd,
) -> Result<Cmd::Output, Cmd::Error>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = BaseG>,
    Cmd::Error: From<Error>,
{
    amount.with_super_coin(CoinResolve {
        price: CheckedConversion::<BaseG, QuoteC, QuoteG>(amount_quote, PhantomData, PhantomData),
        cmd,
    })
}

trait PriceFactory {
    type G: Group;
    type QuoteC: Currency + MemberOf<Self::QuoteG>;
    type QuoteG: Group;

    fn try_obtain_price<C>(self, amount: Coin<C>) -> Result<Price<C, Self::QuoteC>, Error>
    where
        C: Currency + MemberOf<Self::G>;
}

struct UncheckedConversion<'price, BaseG, QuoteC, QuoteG>(&'price BasePrice<BaseG, QuoteC, QuoteG>)
where
    BaseG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group;

impl<'price, BaseG, QuoteC, QuoteG> PriceFactory
    for UncheckedConversion<'price, BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group,
{
    type G = BaseG;
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn try_obtain_price<C>(self, amount: Coin<C>) -> Result<Price<C, Self::QuoteC>, Error>
    where
        C: Currency + MemberOf<Self::G>,
    {
        Ok(Price::new(amount, self.0.amount_quote))
    }
}

struct CheckedConversion<BaseG, QuoteC, QuoteG>(
    Coin<QuoteC>,
    PhantomData<BaseG>,
    PhantomData<QuoteG>,
)
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group;

impl<BaseG, QuoteC, QuoteG> PriceFactory for CheckedConversion<BaseG, QuoteC, QuoteG>
where
    BaseG: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    type G = BaseG;
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn try_obtain_price<C>(self, amount: Coin<C>) -> Result<Price<C, Self::QuoteC>, Error>
    where
        C: Currency + MemberOf<Self::G>,
    {
        Price::try_new(amount, self.0)
    }
}

struct CoinResolve<Price, G, QuoteC, QuoteG, Cmd>
where
    Price: PriceFactory<G = G, QuoteC = QuoteC, QuoteG = QuoteG>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
{
    price: Price,
    cmd: Cmd,
}

impl<Price, G, QuoteC, QuoteG, Cmd> WithCoin<G> for CoinResolve<Price, G, QuoteC, QuoteG, Cmd>
where
    Price: PriceFactory<G = G, QuoteC = QuoteC, QuoteG = QuoteG>,
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
    Cmd: WithPrice<QuoteC, PriceG = G>,
    Cmd::Error: From<Error>,
{
    type Output = Cmd::Output;

    type Error = Cmd::Error;

    fn on<C>(self, amount: Coin<C>) -> WithCoinResult<G, Self>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G> + MemberOf<G::TopG>,
    {
        self.price
            .try_obtain_price(amount)
            .map_err(Into::into)
            .and_then(|price| self.cmd.exec(price))
    }
}
