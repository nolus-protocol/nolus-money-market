use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{group::MemberOf, Currency, CurrencyDTO, Group};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{self, Error, Result};

use self::impl_::{BasePriceRequest, CheckedConverter, OracleStub, RequestBuilder};

mod impl_;

#[cfg(feature = "unchecked-quote-currency")]
pub fn new_unchecked_quote_currency_stub<'a, G, StableC, StableG>(
    oracle: Addr,
    querier: QuerierWrapper<'a>,
) -> impl Oracle<QuoteC = StableC, QuoteG = StableG> + 'a
where
    G: Group,
    StableC: Currency + MemberOf<StableG>,
    StableG: Group + 'a,
{
    use self::impl_::QuoteCUncheckedConverter;
    use self::impl_::StablePriceRequest;

    // TODO pass StablePriceRequest to OracleStub to parameterize it over which currency to use as quote - the base or the stable
    impl_::OracleStub::<
        G,
        StableC,
        StableG,
        StablePriceRequest,
        QuoteCUncheckedConverter<G, StableC, StableG>,
    >::new(OracleRef::unchecked(oracle), querier)
}

pub trait Oracle
where
    Self:
        Into<OracleRef<Self::QuoteC, Self::QuoteG>> + AsRef<OracleRef<Self::QuoteC, Self::QuoteG>>,
{
    type G: Group;
    type QuoteC: Currency + MemberOf<Self::QuoteG>;
    type QuoteG: Group;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>, Self::G>
    where
        C: Currency + MemberOf<Self::G>;
}

pub trait WithOracle<OracleBase> {
    type Output;
    type Error;

    fn exec<O>(self, oracle: O) -> StdResult<Self::Output, Self::Error>
    where
        O: Oracle<QuoteC = OracleBase>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO get back to deny unknown fields once all leases have passed through read/write cycle #[serde(deny_unknown_fields, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub struct OracleRef<QuoteC, QuoteG> {
    addr: Addr,
    #[serde(skip)]
    _quote: PhantomData<QuoteC>,
    #[serde(skip)]
    _quote_group: PhantomData<QuoteG>,
}

impl<QuoteC, QuoteG> OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn try_from_base(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self, QuoteG> {
        Self::try_from::<BasePriceRequest>(addr, querier)
    }

    pub fn try_from<CurrencyReq>(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self, QuoteG>
    where
        CurrencyReq: RequestBuilder,
    {
        querier
            .query_wasm_smart(addr.clone(), &CurrencyReq::currency::<QuoteG>())
            .map_err(Error::StubConfigQuery)
            .and_then(|quote_c: CurrencyDTO<QuoteG>| {
                currency_check(
                    quote_c,
                    CurrencyDTO::<QuoteG>::from_currency_type::<QuoteC>(),
                )
            })
            .map(|()| Self::unchecked(addr))
    }

    pub fn execute_as_oracle<G, V>(
        self,
        cmd: V,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        G: Group,
        V: WithOracle<QuoteC>,
    {
        cmd.exec(OracleStub::<
            G,
            QuoteC,
            QuoteG,
            BasePriceRequest,
            CheckedConverter<G, QuoteC, QuoteG>,
        >::new(self, querier))
    }
}

impl<QuoteC, QuoteG> OracleRef<QuoteC, QuoteG> {
    pub fn unchecked(addr: Addr) -> Self {
        Self {
            addr,
            _quote: PhantomData,
            _quote_group: PhantomData,
        }
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }
}

fn currency_check<G>(got: CurrencyDTO<G>, expected: CurrencyDTO<G>) -> Result<(), G>
where
    G: Group,
{
    if got == expected {
        Ok(())
    } else {
        Err(error::currency_mismatch(expected, got))
    }
}
