use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolOwned};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{self, Error, Result};

use self::impl_::{BasePriceRequest, CheckedConverter, OracleStub, RequestBuilder};

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
    use self::impl_::StablePriceRequest;

    impl_::OracleStub::<StableC, StableG, StablePriceRequest, QuoteCUncheckedConverter>::new(
        OracleRef::unchecked(oracle),
        querier,
    )
}

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
    OracleBase: ?Sized,
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

impl<QuoteC> OracleRef<QuoteC>
where
    QuoteC: Currency,
{
    pub fn try_from_base(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        Self::try_from::<BasePriceRequest>(addr, querier)
    }

    pub fn try_from<CurrencyReq>(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self>
    where
        CurrencyReq: RequestBuilder,
    {
        querier
            .query_wasm_smart(addr.clone(), &CurrencyReq::currency())
            .map_err(Error::StubConfigQuery)
            .and_then(|base_c: SymbolOwned| {
                currency::validate_ticker::<QuoteC>(&base_c)
                    .map_err(|_e| error::currency_mismatch::<QuoteC>(base_c))
            })
            .map(|()| Self::unchecked(addr))
    }

    pub fn execute_as_oracle<OracleQuoteG, V>(
        self,
        cmd: V,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleQuoteG: Group,
        V: WithOracle<QuoteC>,
        Error: Into<V::Error>,
    {
        cmd.exec(OracleStub::<
            _,
            OracleQuoteG,
            BasePriceRequest,
            CheckedConverter,
        >::new(self, querier))
    }
}

impl<QuoteC> OracleRef<QuoteC> {
    pub fn unchecked(addr: Addr) -> Self {
        Self {
            addr,
            _quote: PhantomData,
        }
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }
}
