use std::marker::PhantomData;

use core::result::Result as StdResult;
use cosmwasm_std::{Addr, Api, QuerierWrapper};

use finance::currency::{visit_any, AnyVisitor, Currency, SymbolOwned};
use marketprice::storage::Denom;
use platform::batch::Batch;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    error::ContractError,
    msg::{ConfigResponse, PriceResponse, QueryMsg},
};

pub type Result<T> = StdResult<T, ContractError>;

pub trait Oracle<Lpn>: Into<Batch>
where
    Lpn: Currency,
{
    fn get_price(&self, denoms: Vec<Denom>) -> Result<PriceResponse>;
}

pub trait WithOracle {
    type Output;
    type Error;

    fn exec<C: 'static, L>(self, lpp: L) -> StdResult<Self::Output, Self::Error>
    where
        L: Oracle<C>,
        C: Currency + Serialize;

    fn unknown_lpn(self, symbol: SymbolOwned) -> StdResult<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleRef {
    addr: Addr,
    currency: SymbolOwned,
}

impl OracleRef {
    pub fn try_from<A>(addr_raw: String, api: &A, querier: &QuerierWrapper) -> Result<Self>
    where
        A: ?Sized + Api,
    {
        let addr = api.addr_validate(&addr_raw)?;
        let resp: ConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config {})?;
        let currency = resp.base_asset;
        Ok(Self { addr, currency })
    }

    pub fn execute<V, O, E>(&self, cmd: V, querier: &QuerierWrapper) -> StdResult<O, E>
    where
        V: WithOracle<Output = O, Error = E>,
    {
        struct CurrencyVisitor<'a, V, O, E>
        where
            V: WithOracle<Output = O, Error = E>,
        {
            cmd: V,
            oracle_ref: &'a OracleRef,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, V, O, E> AnyVisitor for CurrencyVisitor<'a, V, O, E>
        where
            V: WithOracle<Output = O, Error = E>,
        {
            type Output = O;
            type Error = E;

            fn on<C>(self) -> StdResult<Self::Output, Self::Error>
            where
                C: Currency + Serialize + DeserializeOwned + 'static,
            {
                self.cmd.exec(self.oracle_ref.as_stub::<C>(self.querier))
            }

            fn on_unknown(self) -> StdResult<Self::Output, Self::Error> {
                self.cmd.unknown_lpn(self.oracle_ref.currency.clone())
            }
        }
        visit_any(
            &self.currency,
            CurrencyVisitor {
                cmd,
                oracle_ref: self,
                querier,
            },
        )
    }

    fn as_stub<'a, C>(&'a self, querier: &'a QuerierWrapper) -> OracleStub<'a, C> {
        OracleStub {
            addr: self.addr.clone(),
            currency: PhantomData::<C>,
            querier,
            batch: Batch::default(),
        }
    }
}

#[cfg(feature = "testing")]
impl OracleRef {
    pub fn unchecked<A, Lpn>(addr: A) -> Self
    where
        A: Into<String>,
        Lpn: Currency,
    {
        Self {
            addr: Addr::unchecked(addr),
            currency: Lpn::SYMBOL.into(),
        }
    }
}

struct OracleStub<'a, C> {
    addr: Addr,
    currency: PhantomData<C>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, Lpn> Oracle<Lpn> for OracleStub<'a, Lpn>
where
    Lpn: Currency + DeserializeOwned,
{
    fn get_price(&self, denoms: Vec<Denom>) -> Result<PriceResponse> {
        let msg = QueryMsg::PriceFor { denoms };
        self.querier
            .query_wasm_smart(self.addr.clone(), &msg)
            .map_err(ContractError::from)
    }
}

impl<'a, C> From<OracleStub<'a, C>> for Batch {
    fn from(stub: OracleStub<'a, C>) -> Self {
        stub.batch
    }
}
