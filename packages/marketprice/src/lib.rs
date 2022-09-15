use std::marker::PhantomData;

use cosmwasm_std::Storage;
use error::PriceFeedsError;
use finance::{
    coin::Coin,
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned},
    price::{
        self,
        dto::{with_base::execute, PriceDTO},
        dto::{WithBase, WithPrice},
        Price,
    },
};
use market_price::{Parameters, PriceFeeds};
use serde::{de::DeserializeOwned, Serialize};

pub mod alarms;
pub mod error;
pub mod feed;
pub mod feeders;
pub mod market_price;

#[cfg(test)]
mod tests;

const MARKET_PRICE: PriceFeeds<'static> = PriceFeeds::new("market_price");

pub struct WithQuote<'a> {
    storage: &'a dyn Storage,
    base: SymbolOwned,
    quote: SymbolOwned,
    parameters: Parameters,
}

impl<'a> WithQuote<'a> {
    pub fn cmd(
        storage: &'a dyn Storage,
        base: SymbolOwned,
        quote: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let visitor = Self {
            storage,
            base,
            quote,
            parameters,
        };

        visit_any(&visitor.quote.clone(), visitor)
    }
}

impl<'a> AnyVisitor for WithQuote<'a> {
    type Output = PriceDTO;
    type Error = PriceFeedsError;

    fn on<QuoteC>(self) -> Result<Self::Output, Self::Error>
    where
        QuoteC: 'static + Currency + DeserializeOwned + Serialize,
    {
        Ok(PriceForCurrency::<QuoteC>::cmd(
            self.storage,
            self.base,
            self.parameters,
        )?)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}

pub struct PriceForCurrency<'a, QuoteC> {
    storage: &'a dyn Storage,
    parameters: Parameters,
    currency: SymbolOwned,
    _oracle_base: PhantomData<QuoteC>,
}

impl<'a, QuoteC> PriceForCurrency<'a, QuoteC>
where
    QuoteC: Currency,
{
    pub fn cmd(
        storage: &'a dyn Storage,
        currency: SymbolOwned,
        parameters: Parameters,
    ) -> Result<PriceDTO, PriceFeedsError> {
        let visitor = Self {
            storage,
            parameters,
            currency,
            _oracle_base: PhantomData,
        };
        visit_any(&visitor.currency.clone(), visitor)
    }
}

impl<'a, QuoteC> AnyVisitor for PriceForCurrency<'a, QuoteC>
where
    QuoteC: Currency,
{
    type Output = PriceDTO;
    type Error = PriceFeedsError;

    fn on<BaseC>(self) -> Result<Self::Output, Self::Error>
    where
        BaseC: 'static + Currency + DeserializeOwned + Serialize,
    {
        // check if both currencies are the same => return one
        if BaseC::SYMBOL.to_string().eq(&QuoteC::SYMBOL.to_string()) {
            let price: Price<BaseC, QuoteC> =
                price::total_of(Coin::<BaseC>::new(1)).is(Coin::<QuoteC>::new(1));

            return Ok(PriceDTO::try_from(price)?);
        }

        // check for exact match for the denom pair
        Ok(MARKET_PRICE.load(
            self.storage,
            BaseC::SYMBOL.to_string(),
            QuoteC::SYMBOL.to_string(),
            self.parameters,
        )?)
    }
    fn on_unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}

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
        println!("-----------------------------------");
        println!("C base {}", C::SYMBOL.to_string());
        println!("QuoteC quote {}", QuoteC::SYMBOL.to_string());
        println!("p2 base  {}", self.p2.base().symbol().to_string());
        println!("p2 quote  {}", self.p2.quote().symbol().to_string());
        println!("-----------------------------------");

        execute(self.p2, Multiplier::new(p1))
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
        let res: Price<C1, QuoteC2> = self.p1.lossy_mul(p2);

        return Ok(PriceDTO::try_from(res)?);
    }

    fn unknown(self) -> Result<Self::Output, Self::Error> {
        Err(PriceFeedsError::UnknownCurrency {})
    }
}
