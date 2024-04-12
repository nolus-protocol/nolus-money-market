use std::{fmt::Debug, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{Currency, Group, SymbolOwned};
use finance::price::Price;
use sdk::cosmwasm_std::{Addr, QuerierWrapper};

use self::{
    error::{Error, Result},
    impl_::OracleStub,
};

use super::error;

use crate::api::price::QueryMsg;

pub mod convert;
mod impl_;

pub trait Oracle<OracleBase>
where
    Self: Into<OracleRef<OracleBase>>,
    OracleBase: ?Sized,
{
    fn price_of<C, G>(&self) -> Result<Price<C, OracleBase>>
    where
        C: ?Sized + Currency,
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
pub struct OracleRef<BaseC>
where
    BaseC: ?Sized,
{
    addr: Addr,
    #[serde(skip)]
    _base: PhantomData<BaseC>,
}

impl<BaseC> OracleRef<BaseC>
where
    BaseC: ?Sized,
{
    pub fn addr(&self) -> &Addr {
        &self.addr
    }

    fn new(addr: Addr) -> Self {
        Self {
            addr,
            _base: PhantomData,
        }
    }
}

impl<BaseC> OracleRef<BaseC>
where
    BaseC: ?Sized + Currency,
{
    pub fn try_from(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        querier
            .query_wasm_smart(addr.clone(), &QueryMsg::BaseCurrency {})
            .map_err(Error::StubConfigQuery)
            .and_then(|base_c| {
                currency::validate_ticker::<BaseC>(base_c).map_err(Error::CurrencyMismatch)
            })
            .map(|_: SymbolOwned| Self::new(addr))
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        self.addr == contract
    }

    // TODO review if the OracleBase Group type is needed anymore once refactor currencies to
    // point to their group
    pub fn execute_as_oracle<OracleBaseG, V>(
        self,
        cmd: V,
        querier: QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleBaseG: Group,
        V: WithOracle<BaseC>,
        Error: Into<V::Error>,
    {
        cmd.exec(OracleStub::<_, OracleBaseG>::new(self, querier))
    }
}

#[cfg(feature = "testing")]
impl<BaseC> OracleRef<BaseC> {
    pub fn unchecked<A>(addr: A) -> Self
    where
        A: Into<String>,
    {
        Self::new(Addr::unchecked(addr))
    }
}
