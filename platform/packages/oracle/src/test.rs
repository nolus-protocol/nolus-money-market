use std::marker::PhantomData;

use currency::{Currency, CurrencyDef, Group, MemberOf};
use finance::{
    coin::Amount,
    price::{self, Price},
};
use sdk::cosmwasm_std::{Addr, StdError};

use crate::{
    OracleRef,
    error::{self, Result},
    stub::Oracle,
};

pub struct DummyOracle<G, QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    price: Option<Amount>,
    oracle_ref: OracleRef<QuoteC, QuoteG>,
    _group: PhantomData<G>,
}

impl<G, QuoteC, QuoteG> DummyOracle<G, QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn with_price(c_in_base: Amount) -> Self {
        Self {
            price: Some(c_in_base),
            oracle_ref: Self::ref_(),
            _group: PhantomData,
        }
    }

    pub fn failing() -> Self {
        Self {
            price: None,
            oracle_ref: Self::ref_(),
            _group: PhantomData,
        }
    }

    fn ref_() -> OracleRef<QuoteC, QuoteG> {
        OracleRef::unchecked(Addr::unchecked("ADDR"))
    }
}

impl<CurrencyG, G, QuoteC, QuoteG> Oracle<CurrencyG> for DummyOracle<G, QuoteC, QuoteG>
where
    CurrencyG: Group + MemberOf<G>,
    G: Group,
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
    QuoteG: Group,
{
    type QuoteC = QuoteC;
    type QuoteG = QuoteG;

    fn price_of<C>(&self) -> Result<Price<C, QuoteC>>
    where
        C: CurrencyDef,
        C::Group: MemberOf<CurrencyG>,
    {
        self.price
            .map(|price| price::total_of(1.into()).is(price.into()))
            .ok_or_else(|| {
                error::failed_to_fetch_price(
                    C::dto(),
                    QuoteC::dto(),
                    StdError::msg("Test failing Oracle::price_of()"),
                )
            })
    }
}

impl<G, QuoteC, QuoteG> From<DummyOracle<G, QuoteC, QuoteG>> for OracleRef<QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn from(value: DummyOracle<G, QuoteC, QuoteG>) -> Self {
        value.oracle_ref
    }
}

impl<G, QuoteC, QuoteG> AsRef<OracleRef<QuoteC, QuoteG>> for DummyOracle<G, QuoteC, QuoteG>
where
    G: Group,
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn as_ref(&self) -> &OracleRef<QuoteC, QuoteG> {
        &self.oracle_ref
    }
}
