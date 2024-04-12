use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::{
    alarms::{Alarm, AlarmCurrencies, Error, ExecuteMsg, Result},
    BaseCurrencies,
};

use super::OracleRef;

pub trait PriceAlarms
where
    Self: Into<Batch>,
{
    type BaseC;

    //TODO use a type-safe Alarm, one with the typed Price
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, BaseCurrencies>) -> Result<()>;
}

pub trait AsAlarms {
    fn as_alarms(&self) -> impl PriceAlarms;
}

impl<OracleBase> AsAlarms for OracleRef<OracleBase> {
    fn as_alarms(&self) -> impl PriceAlarms {
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

impl<'a, OracleBase> PriceAlarms for AlarmsStub<'a, OracleBase> {
    type BaseC = OracleBase;

    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, BaseCurrencies>) -> Result<()> {
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

impl<'a, OracleBase> From<AlarmsStub<'a, OracleBase>> for Batch {
    fn from(stub: AlarmsStub<'a, OracleBase>) -> Self {
        stub.batch
    }
}
