#![cfg(feature = "testing")]

use currency::{Currency, Group};
use finance::{
    coin::Amount,
    price::{self, Price},
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    error::{Error, Result},
    stub::Oracle,
    OracleRef,
};

pub struct DummyOracle<QuoteC>(Option<Amount>, OracleRef<QuoteC>);

impl<QuoteC> DummyOracle<QuoteC> {
    pub fn with_price(c_in_base: Amount) -> Self {
        Self(Some(c_in_base), Self::ref_())
    }

    pub fn failing() -> Self {
        Self(None, Self::ref_())
    }

    fn ref_() -> OracleRef<QuoteC> {
        OracleRef::unchecked(Addr::unchecked("ADDR"))
    }
}

impl<QuoteC> Oracle<QuoteC> for DummyOracle<QuoteC>
where
    QuoteC: Currency,
{
    fn price_of<C, G>(&self) -> Result<Price<C, QuoteC>>
    where
        C: Currency,
        G: Group,
    {
        self.0
            .map(|price| price::total_of(1.into()).is(price.into()))
            .ok_or_else(|| Error::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: QuoteC::TICKER.into(),
                error: StdError::GenericErr {
                    msg: "Test failing Oracle::price_of()".into(),
                },
            })
    }
}

impl<QuoteC> From<DummyOracle<QuoteC>> for OracleRef<QuoteC> {
    fn from(value: DummyOracle<QuoteC>) -> Self {
        value.1
    }
}

impl<QuoteC> AsRef<OracleRef<QuoteC>> for DummyOracle<QuoteC> {
    fn as_ref(&self) -> &OracleRef<QuoteC> {
        &self.1
    }
}
