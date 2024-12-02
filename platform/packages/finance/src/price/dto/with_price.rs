use std::{marker::PhantomData, result::Result as StdResult};

use currency::{
    error::Error as CurrencyError, AnyVisitorPair, AnyVisitorPairResult, Currency, CurrencyDTO,
    Group, InPoolWith, MemberOf,
};

use crate::{
    coin::{Coin, CoinDTO},
    error::{Error, Result},
    price::Price,
};

use super::{PriceDTO, WithPrice};

/// Execute the provided price command on a valid price
pub fn execute<G, Cmd>(price: &PriceDTO<G>, cmd: Cmd) -> StdResult<Cmd::Output, Cmd::Error>
where
    G: Group<TopG = G>,
    Cmd: WithPrice<G = G>,
    Cmd::Error: From<Error> + From<CurrencyError>,
{
    // the refactored code that substituted the Price generic parameter with an enum Price got worse in the size of the output .wasm
    // trait objects are not possible here due to the generic function parameters
    currency::visit_any_on_currencies(
        price.amount.currency(),
        price.amount_quote.currency(),
        PriceAmountVisitor {
            amount: &price.amount,
            amount_quote: &price.amount_quote,
            price: NonValidatingPrice {
                _amount_g: PhantomData::<G>,
            },
            cmd,
        },
    )
}

/// Execute the provided price command on a non-validated price
/// Intended mainly for invariant validation purposes.
pub(super) fn execute_with_coins<G, Cmd>(
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<G>,
    cmd: Cmd,
) -> StdResult<Cmd::Output, Cmd::Error>
where
    G: Group<TopG = G>,
    Cmd: WithPrice<G = G>,
    Cmd::Error: From<Error> + From<CurrencyError>,
{
    currency::visit_any_on_currencies(
        amount.currency(),
        amount_quote.currency(),
        PriceAmountVisitor {
            amount: &amount,
            amount_quote: &amount_quote,
            price: ValidatingPrice {
                _amount_g: PhantomData::<G>,
            },
            cmd,
        },
    )
}

struct PriceAmountVisitor<'amount, G, Price, Cmd>
where
    G: Group,
{
    amount: &'amount CoinDTO<G>,
    amount_quote: &'amount CoinDTO<G>,
    price: Price,
    cmd: Cmd,
}

impl<G, Price, Cmd> AnyVisitorPair for PriceAmountVisitor<'_, G, Price, Cmd>
where
    G: Group<TopG = G>,
    Price: PriceFactory<G = G>,
    Cmd: WithPrice<G = G>,
    Cmd::Error: From<Error>,
{
    type VisitedG = G;

    type Output = Cmd::Output;

    type Error = Cmd::Error;

    fn on<C1, C2>(
        self,
        dto1: &CurrencyDTO<Self::VisitedG>,
        dto2: &CurrencyDTO<Self::VisitedG>,
    ) -> AnyVisitorPairResult<Self>
    where
        C1: Currency + MemberOf<Self::VisitedG>,
        C2: Currency + MemberOf<Self::VisitedG> + InPoolWith<C1>,
    {
        self.price
            .try_obtain_price::<C1, C2>(
                self.amount.as_specific(dto1),
                self.amount_quote.as_specific(dto2),
            )
            .map_err(Into::into)
            .and_then(|price| self.cmd.exec(price))
    }
}

pub trait PriceFactory {
    type G: Group;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::G>;
}

struct NonValidatingPrice<G> {
    _amount_g: PhantomData<G>,
}
impl<G> PriceFactory for NonValidatingPrice<G>
where
    G: Group,
{
    type G = G;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::G>,
    {
        Ok(Price::new(amount, amount_quote))
    }
}

struct ValidatingPrice<G> {
    _amount_g: PhantomData<G>,
}

impl<G> PriceFactory for ValidatingPrice<G>
where
    G: Group,
{
    type G = G;

    fn try_obtain_price<C, QuoteC>(
        self,
        amount: Coin<C>,
        amount_quote: Coin<QuoteC>,
    ) -> Result<Price<C, QuoteC>>
    where
        C: Currency + MemberOf<Self::G>,
        QuoteC: Currency + MemberOf<Self::G>,
    {
        Price::try_new(amount, amount_quote)
    }
}
