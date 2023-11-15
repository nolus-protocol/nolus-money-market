use std::{fmt::Debug, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolOwned};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use crate::{
    error::{self, Error, Result},
    msg::{Config, QueryMsg},
};

use self::impl_::{CheckedConverter, OracleStub};

mod impl_;

#[cfg(feature = "unchecked-base-currency")]
pub fn new_unchecked_base_currency_stub<'a, OracleBase, OracleBaseG>(
    oracle: Addr,
    querier: QuerierWrapper<'a>,
) -> impl Oracle<OracleBase> + 'a
where
    OracleBase: Currency,
    OracleBaseG: Group + for<'de> Deserialize<'de> + 'a,
{
    use self::impl_::BaseCUncheckedConverter;

    OracleStub::<OracleBase, OracleBaseG, BaseCUncheckedConverter>::new(
        OracleRef::new(oracle, OracleBase::TICKER.into()),
        querier,
    )
}

pub trait Oracle<OracleBase>
where
    Self: Into<OracleRef> + AsRef<Self>,
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
    pub fn try_from(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        querier
            .query_wasm_smart(addr.clone(), &QueryMsg::Config {})
            .map_err(Error::StubConfigQuery)
            .map(|resp: Config| Self::new(addr, resp.base_asset))
    }

    fn new(addr: Addr, base_currency: SymbolOwned) -> Self {
        Self {
            addr,
            base_currency,
        }
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
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleBase: Currency,
        OracleBaseG: Group + for<'de> Deserialize<'de>,
        V: WithOracle<OracleBase>,
        Error: Into<V::Error>,
    {
        self.check_base::<OracleBase>();
        cmd.exec(OracleStub::<OracleBase, OracleBaseG, CheckedConverter>::new(self, querier))
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
