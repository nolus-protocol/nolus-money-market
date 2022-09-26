use currency::payment::PaymentGroup;
use error::PriceFeedsError;
use finance::{
    currency::Currency,
    price::{
        dto::{with_base::execute, PriceDTO},
        dto::{WithBase, WithPrice},
        Price,
    },
};
use serde::{de::DeserializeOwned, Serialize};

pub mod alarms;
pub mod error;
pub mod feed;
pub mod feeders;
pub mod market_price;

#[cfg(test)]
mod tests;

pub struct Multiply {
    p2: PriceDTO,
}

impl Multiply {
    fn with(p2: PriceDTO) -> Self {
        Self { p2 }
    }
}

impl WithPrice for Multiply {
    type Output = PriceDTO;

    type Error = PriceFeedsError;

    fn exec<C, QuoteC>(self, p1: Price<C, QuoteC>) -> Result<Self::Output, Self::Error>
    where
        C: 'static + Currency + DeserializeOwned + Serialize,
        QuoteC: 'static + Currency + DeserializeOwned + Serialize,
    {
        execute::<PaymentGroup, Multiplier<C, QuoteC>, QuoteC>(self.p2, Multiplier::new(p1))
    }

    fn unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}

pub struct Multiplier<C1, QuoteC1>
where
    C1: Currency + Serialize + DeserializeOwned,
    QuoteC1: Currency,
{
    p1: Price<C1, QuoteC1>,
}

impl<C1, QuoteC1> Multiplier<C1, QuoteC1>
where
    C1: Currency + Serialize + DeserializeOwned,
    QuoteC1: Currency,
{
    fn new(p: Price<C1, QuoteC1>) -> Self {
        Self { p1: p }
    }
}

impl<C1, QuoteC1> WithBase<QuoteC1> for Multiplier<C1, QuoteC1>
where
    C1: Currency + Serialize + DeserializeOwned,
    QuoteC1: Currency,
{
    type Output = PriceDTO;

    type Error = PriceFeedsError;

    fn exec<QuoteC2>(self, p2: Price<QuoteC1, QuoteC2>) -> Result<Self::Output, Self::Error>
    where
        QuoteC2: Currency,
    {
        Ok(PriceDTO::try_from(self.p1.lossy_mul(p2))?)
    }

    fn unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}
