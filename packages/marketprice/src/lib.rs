use std::marker::PhantomData;

use cosmwasm_std::Storage;
use error::PriceFeedsError;
use finance::{
    coin::Coin,
    currency::{visit_any, AnyVisitor, Currency, SymbolOwned},
    price::{self, Price, PriceDTO},
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
        Ok(PriceFeeds::new("market_price").load(
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
