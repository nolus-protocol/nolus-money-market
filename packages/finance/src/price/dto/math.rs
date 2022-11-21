use std::marker::PhantomData;

use crate::{
    currency::{Currency, Group},
    error::Error,
    price::{
        dto::{PriceDTO, WithBase, WithPrice},
        Price,
    },
};

use super::with_base;

pub struct Multiply<'a, G1, QuoteG1, QuoteG2>
where
    G1: Group,
    QuoteG1: Group,
    QuoteG2: Group,
{
    p2: &'a PriceDTO<QuoteG1, QuoteG2>,
    _g1: PhantomData<G1>,
}

impl<'a, G1, QuoteG1, QuoteG2> Multiply<'a, G1, QuoteG1, QuoteG2>
where
    G1: Group,
    QuoteG1: Group,
    QuoteG2: Group,
{
    pub fn with(p2: &'a PriceDTO<QuoteG1, QuoteG2>) -> Self {
        Self {
            p2,
            _g1: PhantomData,
        }
    }
}

impl<'a, G1, QuoteG1, QuoteG2> WithPrice for Multiply<'a, G1, QuoteG1, QuoteG2>
where
    G1: Group,
    QuoteG1: Group,
    QuoteG2: Group,
{
    type Output = PriceDTO<G1, QuoteG2>;

    type Error = Error;

    fn exec<C, QuoteC>(self, p1: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
        QuoteC: Currency,
    {
        with_base::execute(self.p2, Multiplier::new(p1))
    }
}

pub struct Multiplier<G1, QuoteG2, C1, QuoteC1>
where
    C1: Currency,
    QuoteC1: Currency,
{
    p1: Price<C1, QuoteC1>,
    _g1: PhantomData<G1>,
    _quote_g2: PhantomData<QuoteG2>,
}

impl<G1, QuoteG2, C1, QuoteC1> Multiplier<G1, QuoteG2, C1, QuoteC1>
where
    C1: Currency,
    QuoteC1: Currency,
{
    fn new(p: Price<C1, QuoteC1>) -> Self {
        Self {
            p1: p,
            _g1: PhantomData,
            _quote_g2: PhantomData,
        }
    }
}

impl<G1, QuoteG2, C1, QuoteC1> WithBase<QuoteC1> for Multiplier<G1, QuoteG2, C1, QuoteC1>
where
    G1: Group,
    QuoteG2: Group,
    C1: Currency,
    QuoteC1: Currency,
{
    type Output = PriceDTO<G1, QuoteG2>;

    type Error = Error;

    fn exec<QuoteC2>(self, p2: Price<QuoteC1, QuoteC2>) -> Result<Self::Output, Self::Error>
    where
        QuoteC2: Currency,
    {
        Ok(PriceDTO::from(self.p1.lossy_mul(p2)))
    }
}
