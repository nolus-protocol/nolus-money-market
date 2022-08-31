use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::{Addr, Api, QuerierWrapper, wasm_execute};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use finance::currency::{AnyVisitor, Currency, SymbolOwned, visit_any};
use marketprice::{alarms::Alarm, storage::Denom};
use platform::batch::Batch;

use crate::{
    ContractError,
    msg::{ConfigResponse, ExecuteMsg, PriceResponse, QueryMsg},
};

pub type Result<T> = StdResult<T, ContractError>;

pub trait Oracle<OracleBase>: Into<Batch>
where
    OracleBase: Currency + Serialize,
{
    fn get_price(&self, denom: Denom) -> Result<PriceResponse>;

    fn add_alarm(&mut self, alarm: Alarm) -> Result<()>;
}

pub trait WithOracle {
    type Output;
    type Error;

    fn exec<OracleBase, O>(self, oracle: O) -> StdResult<Self::Output, Self::Error>
    where
        O: Oracle<OracleBase>,
        OracleBase: Currency + Serialize;

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

    pub fn owned_by(&self, addr: &Addr) -> bool {
        &self.addr == addr
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

            fn on<OracleBase>(self) -> StdResult<Self::Output, Self::Error>
            where
                OracleBase: Currency + Serialize + DeserializeOwned + 'static,
            {
                self.cmd
                    .exec(self.oracle_ref.as_stub::<OracleBase>(self.querier))
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

    fn as_stub<'a, OracleBase>(
        &'a self,
        querier: &'a QuerierWrapper,
    ) -> OracleStub<'a, OracleBase> {
        OracleStub {
            addr: self.addr.clone(),
            querier,
            batch: Batch::default(),
            _quote_currency: PhantomData::<OracleBase>,
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

struct OracleStub<'a, OracleBase> {
    addr: Addr,
    // currency: PhantomData<C>,
    _quote_currency: PhantomData<OracleBase>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, OracleBase> Oracle<OracleBase> for OracleStub<'a, OracleBase>
where
    OracleBase: Currency + Serialize,
{
    fn get_price(&self, denom: Denom) -> Result<PriceResponse> {
        let msg = QueryMsg::Price { denom };
        self.querier
            .query_wasm_smart(self.addr.clone(), &msg)
            .map_err(ContractError::from)
    }

    fn add_alarm(&mut self, alarm: Alarm) -> Result<()> {
        self.batch.schedule_execute_no_reply(wasm_execute(
            self.addr.clone(),
            &ExecuteMsg::AddPriceAlarm { alarm },
            vec![],
        )?);

        Ok(())
    }
}

impl<'a, OracleBase> From<OracleStub<'a, OracleBase>> for Batch {
    fn from(stub: OracleStub<'a, OracleBase>) -> Self {
        stub.batch
    }
}
