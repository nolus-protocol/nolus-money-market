use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{Currency, Group};
use finance::price::Price;
use sdk::cosmwasm_std::Addr;
#[cfg(feature = "unchecked-quote-currency")]
use sdk::cosmwasm_std::QuerierWrapper;

use crate::error::Result;

mod impl_;

#[cfg(feature = "unchecked-quote-currency")]
pub fn new_unchecked_quote_currency_stub<'a, StableC, StableG>(
    oracle: Addr,
    querier: QuerierWrapper<'a>,
) -> impl Oracle<StableC> + 'a
where
    StableC: Currency,
    StableG: Group + 'a,
{
    use self::impl_::QuoteCUncheckedConverter;

    impl_::OracleStub::<StableC, StableG, QuoteCUncheckedConverter>::new(
        OracleRef::new(oracle),
        querier,
    )
}

//TODO review the necessity of maintaining Oracle traits in the platform and protocol
pub trait Oracle<QuoteC>
where
    Self: Into<OracleRef<QuoteC>> + AsRef<Self>,
    QuoteC: ?Sized,
{
    fn price_of<C, G>(&self) -> Result<Price<C, QuoteC>>
    where
        C: Currency,
        G: Group;
}

pub trait WithOracle<OracleBase>
where
    OracleBase: Currency,
{
    type Output;
    type Error;

    fn exec<O>(self, oracle: O) -> StdResult<Self::Output, Self::Error>
    where
        O: Oracle<OracleBase>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct OracleRef<QuoteC>
where
    QuoteC: ?Sized,
{
    addr: Addr,
    #[serde(skip)]
    _quote: PhantomData<QuoteC>,
}

impl<QuoteC> OracleRef<QuoteC> {
    pub fn new(addr: Addr) -> Self {
        Self {
            addr,
            _quote: PhantomData,
        }
    }
}
