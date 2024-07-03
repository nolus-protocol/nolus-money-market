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
) -> impl Oracle<QuoteC = StableC, QuoteG = StableG> + 'a
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

pub trait Oracle
where
    Self:
        Into<OracleRef<Self::QuoteC, Self::QuoteG>> + AsRef<OracleRef<Self::QuoteC, Self::QuoteG>>,
{
    type QuoteC: ?Sized;
    type QuoteG: Group;

    fn price_of<C, G>(&self) -> Result<Price<C, Self::QuoteC>>
    where
        C: Currency,
        G: Group;
}

pub trait WithOracle<OracleBase, OracleBaseG>
where
    OracleBase: ?Sized,
    OracleBaseG: Group,
{
    type Output;
    type Error;

    fn exec<O>(self, oracle: O) -> StdResult<Self::Output, Self::Error>
    where
        O: Oracle<QuoteC = OracleBase, QuoteG = OracleBaseG>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO get back to deny unknown fields once all leases have passed through read/write cycle #[serde(deny_unknown_fields, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub struct OracleRef<QuoteC, QuoteG>
where
    QuoteC: ?Sized,
    QuoteG: Group,
{
    addr: Addr,
    #[serde(skip)]
    _quote: PhantomData<QuoteC>,
    #[serde(skip)]
    _quote_g: PhantomData<QuoteG>,
}

impl<QuoteC, QuoteG> OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
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

    pub fn execute_as_oracle<V>(
        self,
        cmd: V,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        V: WithOracle<QuoteC, QuoteG>,
        Error: Into<V::Error>,
    {
        cmd.exec(OracleStub::<_, QuoteG, BasePriceRequest, CheckedConverter>::new(self, querier))
    }
}

impl<QuoteC, QuoteG> OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency,
    QuoteG: Group,
{
    pub fn unchecked(addr: Addr) -> Self {
        Self {
            addr,
            _quote: PhantomData,
            _quote_g: PhantomData,
        }
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }
}
