use oracle_platform::OracleRef;

use currency::{Currency, Group};
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::alarms::{Alarm, Error, ExecuteMsg, Result};

pub trait PriceAlarms<AlarmCurrencies, BaseC, OracleBaseG>
where
    AlarmCurrencies: Group,
    BaseC: Currency + ?Sized,
    OracleBaseG: Group,
    Self: Into<Batch> + Sized,
{
    //TODO use a type-safe Alarm, one with the typed Price
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, BaseC, OracleBaseG>) -> Result<()>;
}

pub trait AsAlarms<OracleBase>
where
    OracleBase: Currency + ?Sized,
{
    fn as_alarms<AlarmCurrencies, OracleBaseG>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBase, OracleBaseG>
    where
        AlarmCurrencies: Group,
        OracleBaseG: Group;
}

impl<OracleBase> AsAlarms<OracleBase> for OracleRef<OracleBase>
where
    OracleBase: Currency + ?Sized,
{
    fn as_alarms<AlarmCurrencies, OracleBaseG>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBase, OracleBaseG>
    where
        AlarmCurrencies: Group,
        OracleBaseG: Group,
    {
        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
        }
    }
}

struct AlarmsStub<'a, OracleBase>
where
    OracleBase: Currency + ?Sized,
{
    oracle_ref: &'a OracleRef<OracleBase>,
    batch: Batch,
}

impl<'a, OracleBase> AlarmsStub<'a, OracleBase>
where
    OracleBase: Currency + ?Sized,
{
    fn addr(&self) -> &Addr {
        self.oracle_ref.addr()
    }
}

impl<'a, AlarmCurrencies, OracleBase, OracleBaseG>
    PriceAlarms<AlarmCurrencies, OracleBase, OracleBaseG> for AlarmsStub<'a, OracleBase>
where
    AlarmCurrencies: Group,
    OracleBase: Currency,
    OracleBaseG: Group,
{
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, OracleBase, OracleBaseG>) -> Result<()> {
        self.batch.schedule_execute_no_reply(
            wasm_execute(
                self.addr().clone(),
                &ExecuteMsg::AddPriceAlarm { alarm },
                vec![],
            )
            .map_err(Error::StubAddAlarm)?,
        );

        Ok(())
    }
}

impl<'a, OracleBase> From<AlarmsStub<'a, OracleBase>> for Batch
where
    OracleBase: Currency + ?Sized,
{
    fn from(stub: AlarmsStub<'a, OracleBase>) -> Self {
        stub.batch
    }
}
