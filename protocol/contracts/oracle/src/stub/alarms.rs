use oracle_platform::OracleRef;

use currency::{Currency, Group};
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::alarms::{Alarm, Error, ExecuteMsg, Result};

pub trait PriceAlarms<AlarmCurrencies, OracleBaseG>
where
    AlarmCurrencies: Group,
    OracleBaseG: Group,
    Self: Into<Batch> + Sized,
{
    type BaseC: Currency + ?Sized;

    //TODO use a type-safe Alarm, one with the typed Price
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, Self::BaseC, OracleBaseG>) -> Result<()>;
}

pub trait AsAlarms<OracleBase>
where
    OracleBase: Currency + ?Sized,
{
    fn as_alarms<AlarmCurrencies, OracleBaseG>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBaseG, BaseC = OracleBase>
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
    ) -> impl PriceAlarms<AlarmCurrencies, OracleBaseG, BaseC = OracleBase>
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

impl<'a, AlarmCurrencies, OracleBase, OracleBaseG> PriceAlarms<AlarmCurrencies, OracleBaseG>
    for AlarmsStub<'a, OracleBase>
where
    AlarmCurrencies: Group,
    OracleBase: Currency,
    OracleBaseG: Group,
{
    type BaseC = OracleBase;

    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, Self::BaseC, OracleBaseG>) -> Result<()> {
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
