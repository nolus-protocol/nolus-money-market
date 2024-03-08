use oracle_platform::OracleRef;
use std::marker::PhantomData;

use serde::Serialize;

use currency::{Currency, Group};
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::alarms::{Alarm, Error, ExecuteMsg, Result};


pub trait PriceAlarms<AlarmCurrencies, BaseCurrency>
where
    BaseCurrency: Currency + Serialize,
    AlarmCurrencies: Group,
    Self: Into<Batch> + Sized,
{
    type BaseC;

    //TODO use a type-safe Alarm, one with the typed Price
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, BaseCurrency>) -> Result<()>;
}

pub trait AsAlarms {
    fn as_alarms<OracleBase, AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBase>
    where
        OracleBase: Currency + Serialize,
        AlarmCurrencies: Group;
}

impl AsAlarms for OracleRef {
    fn as_alarms<OracleBase, AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBase>
    where
        OracleBase: Currency + Serialize,
        AlarmCurrencies: Group,
    {
        self.check_base::<OracleBase>();

        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
        }
    }
}

struct AlarmsStub<'a, OracleBase> {
    oracle_ref: &'a OracleRef<OracleBase>,
    batch: Batch,
}

impl<'a, OracleBase> AlarmsStub<'a, OracleBase> {
    fn addr(&self) -> &Addr {
        self.oracle_ref.addr()
    }
}

impl<'a, OracleBase, AlarmCurrencies> PriceAlarms<AlarmCurrencies, OracleBase>
    for AlarmsStub<'a, OracleBase>
where
    OracleBase: Currency + Serialize,
    AlarmCurrencies: Group,
{
    type BaseC = OracleBase;
    
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, OracleBase>) -> Result<()> {
        self.batch.schedule_execute_no_reply(
            wasm_execute(
                self.addr().clone(),
                &ExecuteMsg::AddPriceAlarm::<OracleBase> { alarm },
                vec![],
            )
            .map_err(Error::StubAddAlarm)?,
        );

        Ok(())
    }
}

impl<'a, OracleBase> From<AlarmsStub<'a, OracleBase>> for Batch {
    fn from(stub: AlarmsStub<'a, OracleBase>) -> Self {
        stub.batch
    }
}
