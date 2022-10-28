use std::marker::PhantomData;

use crate::{
    currency::{Currency, Group},
    error::Error,
    price::{
        dto::{with_base::execute, PriceDTO},
        dto::{WithBase, WithPrice},
        Price,
    },
};

pub struct Multiply<'a, G>
where
    G: Group,
{
    p2: &'a PriceDTO,
    _group: PhantomData<G>,
}

impl<'a, G> Multiply<'a, G>
where
    G: Group,
{
    pub fn with(p2: &'a PriceDTO) -> Self {
        Self {
            p2,
            _group: PhantomData,
        }
    }
}

impl<'a, G> WithPrice for Multiply<'a, G>
where
    G: Group,
{
    type Output = PriceDTO;

    type Error = Error;

    fn exec<C, QuoteC>(self, p1: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: Currency,
        QuoteC: Currency,
    {
        execute::<G, Multiplier<C, QuoteC>, QuoteC>(self.p2, Multiplier::new(p1))
    }
}

pub struct Multiplier<C1, QuoteC1>
where
    C1: Currency,
    QuoteC1: Currency,
{
    p1: Price<C1, QuoteC1>,
}

impl<C1, QuoteC1> Multiplier<C1, QuoteC1>
where
    C1: Currency,
    QuoteC1: Currency,
{
    fn new(p: Price<C1, QuoteC1>) -> Self {
        Self { p1: p }
    }
}

impl<C1, QuoteC1> WithBase<QuoteC1> for Multiplier<C1, QuoteC1>
where
    C1: Currency,
    QuoteC1: Currency,
{
    type Output = PriceDTO;

    type Error = Error;

    fn exec<QuoteC2>(self, p2: Price<QuoteC1, QuoteC2>) -> Result<Self::Output, Self::Error>
    where
        QuoteC2: Currency,
    {
        Ok(PriceDTO::from(self.p1.lossy_mul(p2)))
    }
}
