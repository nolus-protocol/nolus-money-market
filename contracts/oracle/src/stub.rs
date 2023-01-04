use std::{convert::TryInto, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use finance::{
    currency::{self, Currency, SymbolOwned},
    price::Price,
};
use marketprice::{alarms::Alarm, SpotPrice};
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr, QuerierWrapper};
use swap::SwapTarget;

use crate::{
    msg::{ConfigResponse, ExecuteMsg, QueryMsg},
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
    OracleBase: Currency,
{
    fn owned_by(&self, addr: &Addr) -> bool;

    fn price_of<C>(&self) -> Result<Price<C, OracleBase>>
    where
        C: Currency;

    fn add_alarm(&mut self, alarm: Alarm) -> Result<()>;
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
    pub fn try_from(addr: Addr, querier: &QuerierWrapper) -> Result<Self> {
        let resp: ConfigResponse = querier.query_wasm_smart(addr.clone(), &QueryMsg::Config {})?;

        let base_currency = resp.config.base_asset;

        Ok(Self {
            addr,
            base_currency,
        })
    }

    pub fn owned_by(&self, addr: &Addr) -> bool {
        &self.addr == addr
    }

    pub fn execute<OracleBase, V>(
        self,
        cmd: V,
        querier: &QuerierWrapper,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleBase: Currency,
        V: WithOracle<OracleBase>,
        ContractError: Into<V::Error>,
    {
        if OracleBase::TICKER == self.base_currency {
            cmd.exec(self.into_stub::<OracleBase>(querier))
        } else {
            Err(ContractError::CurrencyMismatch {
                expected: OracleBase::TICKER.into(),
                found: self.base_currency,
            }
            .into())
        }
    }

    pub fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: &QuerierWrapper,
    ) -> Result<Vec<SwapTarget>> {
        let msg = QueryMsg::SwapPath { from, to };

        querier
            .query_wasm_smart(self.addr.clone(), &msg)
            .map_err(ContractError::from)
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
            base_currency: C::TICKER.into(),
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
    OracleBase: Currency,
{
    fn owned_by(&self, addr: &Addr) -> bool {
        self.oracle_ref.owned_by(addr)
    }

    fn price_of<C>(&self) -> Result<Price<C, OracleBase>>
    where
        C: Currency,
    {
        if currency::equal::<C, OracleBase>() {
            return Ok(Price::identity());
        }

        let msg = QueryMsg::Price {
            currency: C::TICKER.to_string(),
        };
        let dto: SpotPrice = self
            .querier
            .query_wasm_smart(self.addr().clone(), &msg)
            .map_err(ContractError::from)?;

        Ok(dto.try_into()?)
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
