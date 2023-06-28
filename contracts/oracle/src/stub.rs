use std::{convert::TryInto, marker::PhantomData, result::Result as StdResult};

use serde::{Deserialize, Serialize};

use currency::{self, Currency, SymbolOwned};
use finance::price::Price;
use marketprice::SpotPrice;
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr, QuerierWrapper};
use swap::SwapTarget;

use crate::{
    alarms::Alarm,
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
    Self: Into<OracleRef>,
    OracleBase: Currency,
{
    fn price_of<C>(&self) -> Result<Price<C, OracleBase>>
    where
        C: Currency;
}

pub trait PriceAlarms
where
    Self: Into<Batch>,
{
    //TODO use a type-safe Alarm, one with the typed Price
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

pub trait WithPriceAlarms<OracleBase>
where
    OracleBase: Currency,
{
    type Output;
    type Error;

    fn exec<A>(self, alarms: A) -> StdResult<Self::Output, Self::Error>
    where
        A: PriceAlarms;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OracleRef {
    addr: Addr,
    base_currency: SymbolOwned,
}

impl OracleRef {
    pub fn try_from(contract: Addr, querier: &QuerierWrapper<'_>) -> Result<Self> {
        let resp: ConfigResponse =
            querier.query_wasm_smart(contract.clone(), &QueryMsg::Config {})?;

        let base_currency = resp.config.base_asset;

        Ok(Self {
            addr: contract,
            base_currency,
        })
    }

    pub fn owned_by(&self, contract: &Addr) -> bool {
        &self.addr == contract
    }

    pub fn execute_as_oracle<OracleBase, V>(
        self,
        cmd: V,
        querier: &QuerierWrapper<'_>,
    ) -> StdResult<V::Output, V::Error>
    where
        OracleBase: Currency,
        V: WithOracle<OracleBase>,
        ContractError: Into<V::Error>,
    {
        self.check_base::<OracleBase, _>()?;
        cmd.exec(self.into_oracle_stub::<OracleBase>(querier))
    }

    pub fn as_alarms_stub<OracleBase>(&self) -> AlarmsStub<'_, OracleBase> {
        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
            _quote_currency: PhantomData::<OracleBase>,
        }
    }

    pub fn swap_path(
        &self,
        from: SymbolOwned,
        to: SymbolOwned,
        querier: &QuerierWrapper<'_>,
    ) -> Result<Vec<SwapTarget>> {
        let msg = QueryMsg::SwapPath { from, to };

        querier
            .query_wasm_smart(self.addr.clone(), &msg)
            .map_err(ContractError::from)
    }

    fn check_base<OracleBase, Err>(&self) -> StdResult<(), Err>
    where
        OracleBase: Currency,
        ContractError: Into<Err>,
    {
        if OracleBase::TICKER != self.base_currency {
            Err(ContractError::CurrencyMismatch {
                expected: OracleBase::TICKER.into(),
                found: self.base_currency.clone(),
            }
            .into())
        } else {
            Ok(())
        }
    }

    fn into_oracle_stub<'a, OracleBase>(
        self,
        querier: &'a QuerierWrapper<'a>,
    ) -> OracleStub<'a, OracleBase> {
        OracleStub {
            oracle_ref: self,
            querier,
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
            .map_err(|error| ContractError::FailedToFetchPrice {
                from: C::TICKER.into(),
                to: OracleBase::TICKER.into(),
                error,
            })?;

        Ok(dto.try_into()?)
    }
}

impl<'a, OracleBase> From<OracleStub<'a, OracleBase>> for OracleRef {
    fn from(stub: OracleStub<'a, OracleBase>) -> Self {
        stub.oracle_ref
    }
}

pub struct AlarmsStub<'a, OracleBase> {
    oracle_ref: &'a OracleRef,
    _quote_currency: PhantomData<OracleBase>,
    batch: Batch,
}

impl<'a, OracleBase> AlarmsStub<'a, OracleBase> {
    fn addr(&self) -> &Addr {
        &self.oracle_ref.addr
    }
}

impl<'a, OracleBase> PriceAlarms for AlarmsStub<'a, OracleBase>
where
    OracleBase: Currency,
{
    fn add_alarm(&mut self, alarm: Alarm) -> Result<()> {
        self.batch.schedule_execute_no_reply(wasm_execute(
            self.addr().clone(),
            &ExecuteMsg::AddPriceAlarm { alarm },
            vec![],
        )?);

        Ok(())
    }
}

impl<'a, OracleBase> From<AlarmsStub<'a, OracleBase>> for Batch {
    fn from(stub: AlarmsStub<'a, OracleBase>) -> Self {
        stub.batch
    }
}
