use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{self, Currency, Group, SymbolOwned};
use finance::price::{dto::PriceDTO, Price};
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{self, Error, Result},
    msg::{Config, QueryMsg},
};

pub trait Oracle<OracleBase>
where
    Self: Into<OracleRef>,
    OracleBase: Currency,
{
    fn price_of<C, G>(&self) -> Result<Price<C, OracleBase>>
    where
        C: Currency,
        G: Group + for<'de> Deserialize<'de>;
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
pub struct OracleRef {
    addr: Addr,
    base_currency: SymbolOwned,
}

impl OracleRef {
    pub fn try_from(addr: Addr, querier: &QuerierWrapper<'_>) -> Result<Self> {
        querier
            .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
            .map_err(Error::StubConfigQuery)
            .map(|resp: Config| Self {
                addr,
                base_currency: resp.base_asset,
            })
    }

    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }

    pub fn execute_as_oracle<OracleBase, OracleBaseG, V>(
        self,
        cmd: V,
        querier: &QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleBase: Currency,
        OracleBaseG: Group + for<'de> Deserialize<'de>,
        V: WithOracle<OracleBase>,
        Error: Into<V::Error>,
    {
        self.check_base::<OracleBase>();
        currency::validate_member::<OracleBase, OracleBaseG>()
            .expect("execute OracleRef as Oracle with an appropriate currency and group");
        cmd.exec(self.into_oracle_stub::<OracleBase, OracleBaseG>(querier))
    }

    pub fn check_base<OracleBase>(&self)
    where
        OracleBase: Currency,
    {
        assert_eq!(
            OracleBase::TICKER,
            self.base_currency,
            "Base currency mismatch {}",
            error::currency_mismatch::<OracleBase>(self.base_currency.clone())
        );
    }

    fn into_oracle_stub<'a, OracleBase, OracleBaseG>(
        self,
        querier: &'a QuerierWrapper<'a>,
    ) -> OracleStub<'a, OracleBase, OracleBaseG> {
        OracleStub {
            oracle_ref: self,
            querier,
            _quote_currency: PhantomData,
            _quote_group: PhantomData,
        }
    }
}

#[cfg(feature = "testing")]
impl OracleRef {
    pub fn unchecked<A, C>(addr: A) -> Self
    where
        A: Into<String>,
        C: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            base_currency: C::TICKER.into(),
        }
    }
}

struct OracleStub<'a, OracleBase, OracleBaseG> {
    oracle_ref: OracleRef,
    _quote_currency: PhantomData<OracleBase>,
    _quote_group: PhantomData<OracleBaseG>,
    querier: &'a QuerierWrapper<'a>,
}

impl<'a, OracleBase, OracleBaseG> OracleStub<'a, OracleBase, OracleBaseG> {
    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, OracleBase, OracleBaseG> Oracle<OracleBase> for OracleStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group + for<'de> Deserialize<'de>,
{
    fn price_of<C, G>(&self) -> Result<Price<C, OracleBase>>
    where
        C: Currency,
        G: Group + for<'de> Deserialize<'de>,
    {
        if currency::equal::<C, OracleBase>() {
            return Ok(Price::identity());
        }

        let msg = QueryMsg::Price {
            currency: C::TICKER.to_string(),
        };
        self.querier
            .query_wasm_smart(self.addr().clone(), &msg)
            .map_err(|error| Error::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: OracleBase::TICKER.into(),
                error,
            })
            .and_then(|price: PriceDTO<G, OracleBaseG>| price.try_into().map_err(Into::into))
    }
}

impl<'a, OracleBase, OracleBaseG> From<OracleStub<'a, OracleBase, OracleBaseG>> for OracleRef {
    fn from(stub: OracleStub<'a, OracleBase, OracleBaseG>) -> Self {
        stub.oracle_ref
    }
}
