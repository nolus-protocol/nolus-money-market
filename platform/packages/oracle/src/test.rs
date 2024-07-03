#![cfg(feature = "testing")]

use std::marker::PhantomData;

use currency::{group::MemberOf, Currency, CurrencyDTO, Group};
use finance::{
    coin::Amount,
    price::{self, Price},
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    error::{self, Result},
    stub::Oracle,
    OracleRef,
};

pub struct DummyOracle<G, QuoteC>(
    Option<Amount>,
    OracleRef<QuoteC, QuoteC::Group>,
    PhantomData<G>,
)
where
    QuoteC: Currency;

impl<G, QuoteC> DummyOracle<G, QuoteC>
where
    QuoteC: Currency,
{
    pub fn with_price(c_in_base: Amount) -> Self {
        Self(Some(c_in_base), Self::ref_(), PhantomData)
    }

    pub fn failing() -> Self {
        Self(None, Self::ref_(), PhantomData)
    }

    fn ref_() -> OracleRef<QuoteC, QuoteC::Group> {
        OracleRef::unchecked(Addr::unchecked("ADDR"))
    }
}

impl<G, QuoteC> Oracle for DummyOracle<G, QuoteC>
where
    G: Group,
    QuoteC: Currency,
{
    type G = G;

    type QuoteC = QuoteC;

    type QuoteG = QuoteC::Group;

    fn price_of<C>(&self) -> Result<Price<C, QuoteC>, G>
    where
        C: Currency + MemberOf<G>,
    {
        self.0
            .map(|price| price::total_of(1.into()).is(price.into()))
            .ok_or_else(|| {
                error::failed_to_fetch_price(
                    CurrencyDTO::<G>::from_currency_type::<C>(),
                    CurrencyDTO::<Self::QuoteG>::from_currency_type::<QuoteC>(),
                    StdError::GenericErr {
                        msg: "Test failing Oracle::price_of()".into(),
                    },
                )
            })
    }
}

impl<G, QuoteC> From<DummyOracle<G, QuoteC>> for OracleRef<QuoteC, QuoteC::Group>
where
    QuoteC: Currency,
{
    fn from(value: DummyOracle<G, QuoteC>) -> Self {
        value.1
    }
}

impl<G, QuoteC> AsRef<OracleRef<QuoteC, QuoteC::Group>> for DummyOracle<G, QuoteC>
where
    QuoteC: Currency,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteC::Group> {
        &self.1
    }
}
