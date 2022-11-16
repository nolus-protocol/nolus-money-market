use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use sdk::schemars::{self, JsonSchema};

use crate::{
    coin::CoinDTO,
    currency::{Currency, Group},
    error::Error,
    price::Price,
};

use self::math::Multiply;

pub mod math;
pub mod with_base;
pub mod with_price;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PriceDTO<G, QuoteG> {
    amount: CoinDTO<G>,
    amount_quote: CoinDTO<QuoteG>,
}

impl<G, QuoteG> PriceDTO<G, QuoteG> {
    pub fn new(base: CoinDTO<G>, quote: CoinDTO<QuoteG>) -> Self {
        Self {
            amount: base,
            amount_quote: quote,
        }
    }

    pub const fn base(&self) -> &CoinDTO<G> {
        &self.amount
    }

    pub const fn quote(&self) -> &CoinDTO<QuoteG> {
        &self.amount_quote
    }
}

impl<G, QuoteG> PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    pub fn multiply<QuoteG2>(
        &self,
        other: &PriceDTO<QuoteG, QuoteG2>,
    ) -> Result<PriceDTO<G, QuoteG2>, Error>
    where
        QuoteG2: Group,
    {
        with_price::execute(self, Multiply::with(other))
    }
}

impl<G, QuoteG, C, QuoteC> From<Price<C, QuoteC>> for PriceDTO<G, QuoteG>
where
    C: Currency,
    QuoteC: Currency,
{
    fn from(price: Price<C, QuoteC>) -> Self {
        Self::new(price.amount.into(), price.amount_quote.into())
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<&PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: &PriceDTO<G, QuoteG>) -> Result<Self, Self::Error> {
        Ok(super::total_of((&value.amount).try_into()?).is((&value.amount_quote).try_into()?))
    }
}

impl<G, QuoteG, C, QuoteC> TryFrom<PriceDTO<G, QuoteG>> for Price<C, QuoteC>
where
    C: Currency,
    QuoteC: Currency,
{
    type Error = Error;

    fn try_from(value: PriceDTO<G, QuoteG>) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl<G, QuoteG> PartialOrd for PriceDTO<G, QuoteG>
where
    G: Group,
    QuoteG: Group,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        struct Comparator<'a, G, QuoteG> {
            other: &'a PriceDTO<G, QuoteG>,
        }

        impl<'a, G, QuoteG> WithPrice for Comparator<'a, G, QuoteG>
        where
            G: PartialEq,
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

pub trait WithPrice {
    type Output;
    type Error;

    fn exec<C, QuoteC>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
        QuoteC: Currency;
}

pub trait WithBase<C>
where
    C: Currency,
{
    type Output;
    type Error;

    fn exec<QuoteC>(self, _: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        QuoteC: Currency;
}

#[cfg(test)]
mod test {
    use std::cmp::Ordering;

    use crate::{
        coin::Coin,
        error::Error,
        price::{dto::PriceDTO, Price},
        test::currency::{Dai, Nls, TestCurrencies, TestExtraCurrencies, Usdc},
    };

    #[test]
    fn test_multiply() {
        let p1 = PriceDTO::<TestCurrencies, TestExtraCurrencies>::new(
            Coin::<Usdc>::new(10).into(),
            Coin::<Dai>::new(5).into(),
        );
        let p2 = PriceDTO::<TestExtraCurrencies, TestCurrencies>::new(
            Coin::<Dai>::new(20).into(),
            Coin::<Nls>::new(5).into(),
        );

        assert_eq!(
            Ok(Price::new(Coin::<Usdc>::new(8), Coin::<Nls>::new(1)).into()),
            p1.multiply(&p2)
        );
    }
    #[test]
    fn test_multiply_err() {
        let p1 = PriceDTO::<TestCurrencies, TestCurrencies>::new(
            Coin::<Usdc>::new(10).into(),
            Coin::<Dai>::new(5).into(),
        );
        let p2 = PriceDTO::<TestCurrencies, TestCurrencies>::new(
            Coin::<Dai>::new(20).into(),
            Coin::<Nls>::new(5).into(),
        );

        assert!(matches!(
            p1.multiply(&p2),
            Err(Error::NotInCurrencyGroup(_, _))
        ));
    }

    #[test]
    fn test_cmp() {
        let p1: PriceDTO<TestCurrencies, TestExtraCurrencies> =
            Price::new(Coin::<Usdc>::new(20), Coin::<Dai>::new(5000)).into();
        assert!(p1 == p1);
        assert_eq!(Some(Ordering::Equal), p1.partial_cmp(&p1));

        let p2 = Price::new(Coin::<Usdc>::new(20), Coin::<Dai>::new(5001)).into();
        assert!(p1 < p2);
    }

    #[test]
    #[should_panic = "The currencies of both prices should match"]
    fn test_cmp_currencies_mismatch() {
        let p1: PriceDTO<TestCurrencies, TestExtraCurrencies> =
            Price::new(Coin::<Usdc>::new(20), Coin::<Nls>::new(5000)).into();
        let p2 = Price::new(Coin::<Usdc>::new(20), Coin::<Dai>::new(5000)).into();
        let _ = p1 < p2;
    }
}
