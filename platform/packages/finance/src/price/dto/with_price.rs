use std::{marker::PhantomData, result::Result as StdResult};

use currency::{Currency, Group, MemberOf};

use crate::{
    coin::{Coin, CoinDTO, WithCoin, WithCoinResult},
    error::{Error, Result},
    price::Price,
};

use super::{PriceDTO, WithPrice};

/// Execute the provided price command on a valid price
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
    // the refactored code that substituted the Price generic parameter with an enum Price got worse in the size of the output .wasm
    // trait objects are not possible here due to the generic function parameters
    price.amount.with_super_coin(PriceAmountVisitor {
        _amount_g: PhantomData::<G>,
        amount_quote: &price.amount_quote,
        price: NonValidatingPrice {
            _amount_g: PhantomData::<G>,
            _amount_quote_g: PhantomData::<QuoteG>,
        },
        cmd,
    })
}

/// Execute the provided price command on a non-validated price
/// Intended mainly for invariant validation purposes.
pub(super) fn execute_with_coins<G, QuoteG, Cmd>(
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
    amount.with_super_coin(PriceAmountVisitor {
        _amount_g: PhantomData::<G>,
        amount_quote: &amount_quote,
        price: ValidatingPrice {
            _amount_g: PhantomData::<G>,
            _amount_quote_g: PhantomData::<QuoteG>,
        },
        cmd,
    })
}

struct PriceAmountVisitor<'quote, G, QuoteG, Price, Cmd>
where
    G: Group,
    QuoteG: Group,
{
    _amount_g: PhantomData<G>,
    amount_quote: &'quote CoinDTO<QuoteG>,
    price: Price,
    cmd: Cmd,
}

impl<'quote, G, QuoteG, Price, Cmd> WithCoin<G>
    for PriceAmountVisitor<'quote, G, QuoteG, Price, Cmd>
where
    G: Group,
    QuoteG: Group,
    Price: PriceFactory<G = G, QuoteG = QuoteG>,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Cmd::Error: From<Error>,
{
    type Output = Cmd::Output;

    type Error = Cmd::Error;

    fn on<C>(self, amount: Coin<C>) -> WithCoinResult<G, Self>
    where
        C: Currency + MemberOf<G> + MemberOf<G::TopG>,
    {
        self.amount_quote.with_super_coin(PriceQuoteAmountVisitor {
            amount,
            _amount_g: PhantomData::<G>,
            _amount_quote_g: PhantomData::<QuoteG>,
            price: self.price,
            cmd: self.cmd,
        })
    }
}

struct PriceQuoteAmountVisitor<C, G, QuoteG, Price, Cmd>
where
    C: Currency,
    QuoteG: Group,
{
    amount: Coin<C>,
    _amount_g: PhantomData<G>,
    _amount_quote_g: PhantomData<QuoteG>,
    price: Price,
    cmd: Cmd,
}

impl<C, G, QuoteG, Price, Cmd> WithCoin<QuoteG>
    for PriceQuoteAmountVisitor<C, G, QuoteG, Price, Cmd>
where
    C: Currency + MemberOf<G>,
    G: Group,
    QuoteG: Group,
    Price: PriceFactory<G = G, QuoteG = QuoteG>,
    Cmd: WithPrice<G = G, QuoteG = QuoteG>,
    Cmd::Error: From<Error>,
{
    type Output = Cmd::Output;

    type Error = Cmd::Error;

    fn on<QuoteC>(self, amount_quote: Coin<QuoteC>) -> WithCoinResult<QuoteG, Self>
    where
        QuoteC: Currency + MemberOf<QuoteG> + MemberOf<QuoteG::TopG>,
    {
        self.price
            .try_obtain_price(self.amount, amount_quote)
            .map_err(Into::into)
            .and_then(|price| self.cmd.exec(price))
    }
}

pub trait PriceFactory {
    type G: Group;
    type QuoteG: Group;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>;
}

struct NonValidatingPrice<G, QuoteG> {
    _amount_g: PhantomData<G>,
    _amount_quote_g: PhantomData<QuoteG>,
}
impl<G, QuoteG> PriceFactory for NonValidatingPrice<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    type G = G;
    type QuoteG = QuoteG;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>,
    {
        Ok(Price::new(amount, amount_quote))
    }
}

struct ValidatingPrice<G, QuoteG> {
    _amount_g: PhantomData<G>,
    _amount_quote_g: PhantomData<QuoteG>,
}

impl<G, QuoteG> PriceFactory for ValidatingPrice<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    type G = G;

    type QuoteG = QuoteG;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::QuoteG>,
    {
        Price::try_new(amount, amount_quote)
    }
}
