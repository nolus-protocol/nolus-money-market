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

pub struct DummyOracle<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    price: Option<Amount>,
    oracle_ref: OracleRef<QuoteC, QuoteG>,
}

impl<QuoteC, QuoteG> DummyOracle<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    pub fn with_price(c_in_base: Amount) -> Self {
        Self {
            price: Some(c_in_base),
            oracle_ref: Self::ref_(),
        }
    }

    pub fn failing() -> Self {
        Self {
            price: None,
            oracle_ref: Self::ref_(),
        }
    }

    fn ref_() -> OracleRef<QuoteC, QuoteG> {
        OracleRef::unchecked(Addr::unchecked("ADDR"))
    }
}

impl<QuoteC, QuoteG> Oracle for DummyOracle<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn price_of<C, G>(&self) -> Result<Price<C, QuoteC>>
    where
        C: Currency,
        G: Group,
    {
        self.price
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

impl<QuoteC, QuoteG> From<DummyOracle<QuoteC, QuoteG>> for OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    fn from(value: DummyOracle<QuoteC, QuoteG>) -> Self {
        value.oracle_ref
    }
}

impl<QuoteC, QuoteG> AsRef<OracleRef<QuoteC, QuoteG>> for DummyOracle<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}
