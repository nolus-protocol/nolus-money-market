use oracle_platform::OracleRef;

use currency::{Currency, Group, MemberOf};
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::alarms::{Alarm, Error, ExecuteMsg, Result};

pub trait PriceAlarms<AlarmCurrencies>
where
    AlarmCurrencies: Group,
    Self: Into<Batch> + Sized,
{
    type BaseC: Currency + MemberOf<Self::BaseG>;
    type BaseG: Group;

    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, Self::BaseC, Self::BaseG>) -> Result<()>;
}

pub trait AsAlarms<OracleBase, OracleBaseG>
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn as_alarms<AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, BaseC = OracleBase, BaseG = OracleBaseG>
    where
        AlarmCurrencies: Group;
}

impl<OracleBase, OracleBaseG> AsAlarms<OracleBase, OracleBaseG>
    for OracleRef<OracleBase, OracleBaseG>
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn as_alarms<AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, BaseC = OracleBase, BaseG = OracleBaseG>
    where
        AlarmCurrencies: Group,
    {
        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
        }
    }
}

struct AlarmsStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
    oracle_ref: &'a OracleRef<OracleBase, OracleBaseG>,
    batch: Batch,
}

impl<'a, OracleBase, OracleBaseG> AlarmsStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
    fn addr(&self) -> &Addr {
        self.oracle_ref.addr()
    }
}

impl<'a, AlarmCurrencies, OracleBase, OracleBaseG> PriceAlarms<AlarmCurrencies>
    for AlarmsStub<'a, OracleBase, OracleBaseG>
where
    AlarmCurrencies: Group,
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    type BaseC = OracleBase;
    type BaseG = OracleBaseG;

    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, Self::BaseC, Self::BaseG>) -> Result<()> {
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

impl<'a, OracleBase, OracleBaseG> From<AlarmsStub<'a, OracleBase, OracleBaseG>> for Batch
where
    OracleBase: Currency,
    OracleBaseG: Group,
{
    fn from(stub: AlarmsStub<'a, OracleBase, OracleBaseG>) -> Self {
        stub.batch
    }
}
