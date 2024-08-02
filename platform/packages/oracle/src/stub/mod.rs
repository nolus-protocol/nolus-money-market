use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{Currency, CurrencyDTO, Group, MemberOf};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{Error, Result};

use self::impl_::{BasePriceRequest, CheckedConverter, OracleStub, RequestBuilder};

mod impl_;

// TODO re-apply #30d5718df57154e98a8296dab9bd34638195801c once introduce CurrencyDTO. That would eliminate the Symbol
// to Currency matching that is currently done when TryInto::<Coin<Stable>>. The PriceDTO deserialization check is
//implemented with the 'match-any' ticker solution at StableCurrencyGroup
#[cfg(feature = "unchecked-stable-quote")]
pub fn new_unchecked_stable_quote_stub<'a, G, StableC, StableG>(
    oracle: Addr,
    querier: QuerierWrapper<'a>,
) -> impl Oracle<G, QuoteC = StableC, QuoteG = StableG> + AsRef<OracleRef<StableC, StableG>> + 'a
where
    G: Group + 'a,
    StableC: Currency + MemberOf<StableG>,
    StableG: Group + 'a,
{
    use self::impl_::QuoteCUncheckedConverter;
    use self::impl_::StablePriceRequest;

    impl_::OracleStub::<G, StableC, StableG, StablePriceRequest, QuoteCUncheckedConverter>::new(
        OracleRef::unchecked(oracle),
        querier,
    )
}

pub trait Oracle<G>
where
    G: Group,
{
    type QuoteC: Currency + MemberOf<Self::QuoteG>;
    type QuoteG: Group;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>>
    where
        C: Currency + MemberOf<G>;
}

pub trait WithOracle<OracleBase, OracleBaseG>
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    type G: Group;

    type Output;
    type Error;

    fn exec<OracleImpl>(self, oracle: OracleImpl) -> StdResult<Self::Output, Self::Error>
    where
        OracleImpl: Oracle<Self::G, QuoteC = OracleBase, QuoteG = OracleBaseG>
            + Into<OracleRef<OracleBase, OracleBaseG>>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO get back to deny unknown fields once all leases have passed through read/write cycle #[serde(deny_unknown_fields, rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub struct OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
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
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn try_from_base(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        Self::try_from::<BasePriceRequest>(addr, querier)
    }

    pub fn execute_as_oracle<V, G>(
        self,
        cmd: V,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        V: WithOracle<QuoteC, QuoteG, G = G>,
        G: Group,
    {
        cmd.exec(OracleStub::<
            '_,
            G,
            QuoteC,
            QuoteG,
            BasePriceRequest,
            CheckedConverter,
        >::new(self, querier))
    }

    fn try_from<CurrencyReq>(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self>
    where
        CurrencyReq: RequestBuilder,
    {
        querier
            .query_wasm_smart(addr.clone(), &CurrencyReq::currency::<QuoteG>())
            .map_err(Error::StubConfigQuery)
            .and_then(|quote_c: CurrencyDTO<QuoteG>| {
                quote_c
                    .of_currency::<QuoteC>()
                    .map_err(Error::StubConfigInvalid)
            })
            .map(|()| Self::new_internal(addr))
    }
}

impl<QuoteC, QuoteG> OracleRef<QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    #[cfg(any(test, feature = "testing", feature = "unchecked-stable-quote"))]
    pub fn unchecked(addr: Addr) -> Self {
        Self::new_internal(addr)
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }

    fn new_internal(addr: Addr) -> Self {
        Self {
            addr,
            _quote: PhantomData,
            _quote_g: PhantomData,
        }
    }
}
