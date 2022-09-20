use std::{marker::PhantomData, result::Result as StdResult};

use cosmwasm_std::{wasm_execute, Addr, QuerierWrapper};
use serde::{Deserialize, Serialize};

use finance::currency::{visit, Currency, SingleVisitor, SymbolOwned};
use marketprice::alarms::Alarm;
use platform::batch::Batch;

use crate::{
    msg::{ConfigResponse, ExecuteMsg, PriceResponse, QueryMsg},
    ContractError,
};

pub type Result<T> = StdResult<T, ContractError>;

pub struct OracleBatch {
    pub oracle_ref: OracleRef,
    pub batch: Batch,
}

pub trait Oracle<OracleBase>
where
    Self: Into<OracleBatch>,
    OracleBase: Currency + Serialize,
{
    fn owned_by(&self, addr: &Addr) -> bool;

    fn get_price<C>(&self) -> Result<PriceResponse>
    where
        C: Currency;

    fn add_alarm(&mut self, alarm: Alarm) -> Result<()>;
}

pub trait WithOracle<OracleBase>
where
    OracleBase: Currency + Serialize,
{
    type Output;
    type Error;

    fn exec<O>(self, oracle: O) -> StdResult<Self::Output, Self::Error>
    where
        O: Oracle<OracleBase>;

    fn unexpected_base(self, symbol: SymbolOwned) -> StdResult<Self::Output, Self::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleRef {
    addr: Addr,
    currency: SymbolOwned,
}

impl From<OracleRef> for Addr {
    fn from(oracle_ref: OracleRef) -> Self {
        oracle_ref.addr
    }
}

impl OracleRef {
    pub fn try_from(addr: Addr, querier: &QuerierWrapper) -> Result<Self> {
        let resp: ConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config {})?;

        let currency = resp.base_asset;

        Ok(Self { addr, currency })
    }

    pub fn owned_by(&self, addr: &Addr) -> bool {
        &self.addr == addr
    }

    pub fn execute<OracleBase, V, O, E>(self, cmd: V, querier: &QuerierWrapper) -> StdResult<O, E>
    where
        OracleBase: Currency + Serialize,
        V: WithOracle<OracleBase, Output = O, Error = E>,
    {
        struct CurrencyVisitor<'a, OracleBase, V, O, E>
        where
            OracleBase: Currency + Serialize,
            V: WithOracle<OracleBase, Output = O, Error = E>,
        {
            cmd: V,
            oracle_ref: OracleRef,
            _oracle_base: PhantomData<OracleBase>,
            querier: &'a QuerierWrapper<'a>,
        }

        impl<'a, OracleBase, V, O, E> SingleVisitor<OracleBase> for CurrencyVisitor<'a, OracleBase, V, O, E>
        where
            OracleBase: Currency + Serialize,
            V: WithOracle<OracleBase, Output = O, Error = E>,
        {
            type Output = O;
            type Error = E;

            fn on(self) -> StdResult<Self::Output, Self::Error> {
                self.cmd
                    .exec(self.oracle_ref.into_stub::<OracleBase>(self.querier))
            }

            fn on_unknown(self) -> StdResult<Self::Output, Self::Error> {
                self.cmd.unexpected_base(self.oracle_ref.currency)
            }
        }

        visit(
            &self.currency.clone(),
            CurrencyVisitor {
                cmd,
                oracle_ref: self,
                _oracle_base: PhantomData,
                querier,
            },
        )
    }

    fn into_stub<'a, OracleBase>(self, querier: &'a QuerierWrapper) -> OracleStub<'a, OracleBase> {
        OracleStub {
            oracle_ref: self,
            querier,
            batch: Batch::default(),
            _quote_currency: PhantomData::<OracleBase>,
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
            currency: C::SYMBOL.into(),
        }
    }
}

struct OracleStub<'a, OracleBase> {
    oracle_ref: OracleRef,
    _quote_currency: PhantomData<OracleBase>,
    querier: &'a QuerierWrapper<'a>,
    batch: Batch,
}

impl<'a, OracleBase> OracleStub<'a, OracleBase> {
    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, OracleBase> Oracle<OracleBase> for OracleStub<'a, OracleBase>
where
    OracleBase: Currency + Serialize,
{
    fn owned_by(&self, addr: &Addr) -> bool {
        self.oracle_ref.owned_by(addr)
    }

    fn get_price<C>(&self) -> Result<PriceResponse>
    where
        C: Currency,
    {
        let msg = QueryMsg::Price {
            currency: C::SYMBOL.to_string(),
        };
        self.querier
            .query_wasm_smart(self.addr().clone(), &msg)
            .map_err(ContractError::from)
    }

    fn add_alarm(&mut self, alarm: Alarm) -> Result<()> {
        self.batch.schedule_execute_no_reply(wasm_execute(
            self.addr().clone(),
            &ExecuteMsg::AddPriceAlarm { alarm },
            vec![],
        )?);

        Ok(())
    }
}

impl<'a, OracleBase> From<OracleStub<'a, OracleBase>> for OracleBatch {
    fn from(stub: OracleStub<'a, OracleBase>) -> Self {
        OracleBatch {
            oracle_ref: stub.oracle_ref,
            batch: stub.batch,
        }
    }
}
