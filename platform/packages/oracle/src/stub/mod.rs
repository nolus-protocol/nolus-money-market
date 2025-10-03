use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

#[cfg(feature = "unchecked-stable-quote")]
use currency::platform::{PlatformGroup, Stable};
use serde::{Deserialize, Serialize};

use currency::{Currency, CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

#[cfg(feature = "unchecked-stable-quote")]
pub use self::impl_::{StablePriceSource, StablePriceStub};
use crate::error::{Error, Result};

use self::impl_::{BasePriceRequest, OracleStub, RequestBuilder};

mod impl_;

#[cfg(feature = "unchecked-stable-quote")]
pub fn new_unchecked_stable_quote_stub<G>(
    oracle: Addr,
    querier: QuerierWrapper<'_>,
) -> Result<impl Oracle<G, QuoteC = Stable, QuoteG = PlatformGroup> + AsRef<StablePriceSource>>
where
    G: Group,
{
    StablePriceStub::try_new(oracle, querier)
}

pub trait Oracle<G>
where
    G: Group,
{
    type QuoteC: Currency + MemberOf<Self::QuoteG>;
    type QuoteG: Group;

    fn price_of<C>(&self) -> Result<Price<C, Self::QuoteC>>
    where
        C: CurrencyDef,
        C::Group: MemberOf<G>;
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
#[cfg_attr(feature = "testing", derive(PartialEq, Eq))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
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
    QuoteC: CurrencyDef,
    QuoteC::Group: MemberOf<QuoteG>,
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
        QuoteC::Group: MemberOf<G::TopG>,
    {
        cmd.exec(OracleStub::<'_, G, QuoteC, QuoteG, BasePriceRequest>::new(
            self, querier,
        ))
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
                    .of_currency(QuoteC::dto())
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
