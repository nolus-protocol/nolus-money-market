use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use access_control::AccessPermission;
use currency::{Currency, CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::error::{Error, Result};

use self::impl_::{BasePriceRequest, OracleStub, RequestBuilder};

mod impl_;

#[cfg(feature = "unchecked-stable-quote")]
pub fn new_unchecked_stable_quote_stub<G, StableC>(
    oracle: Addr,
    querier: QuerierWrapper<'_>,
) -> impl Oracle<G, QuoteC = StableC, QuoteG = G> + AsRef<OracleRef<StableC, G>>
where
    G: Group<TopG = G>,
    StableC: CurrencyDef,
    StableC::Group: MemberOf<G>,
{
    use self::impl_::StablePriceRequest;

    impl_::OracleStub::<G, StableC, G, StablePriceRequest>::new(
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
// TODO get back to deny unknown fields once all leases have passed through read/write cycle
// [06.08.2024] there are still 19 leases in PaidActive that have "base_currency" field. We would proceed once get closed.
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
        // QuoteC::Group: MemberOf<G> + MemberOf<G::TopG>,
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

pub struct OracleDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    oracle_ref: &'a OracleRef<QuoteC, QuoteG>,
}

impl<'a, QuoteC, QuoteG> OracleDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    pub fn new(oracle_ref: &'a OracleRef<QuoteC, QuoteG>) -> Self {
        Self { oracle_ref }
    }
}

impl<'a, QuoteC, QuoteG> AccessPermission for OracleDelivery<'a, QuoteC, QuoteG>
where
    QuoteC: Currency + MemberOf<QuoteG>,
    QuoteG: Group,
{
    fn is_granted_to(&self, caller: &Addr) -> bool {
        self.oracle_ref.owned_by(caller)
    }
}
