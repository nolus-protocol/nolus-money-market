use std::marker::PhantomData;

use currency::Currency;
use oracle_platform::OracleRef;
use platform::batch::Batch;
use sdk::cosmwasm_std::{wasm_execute, Addr};

use crate::api::alarms::{Alarm, AlarmCurrencies, Error, ExecuteMsg, Result, StableCurrency};

pub trait PriceAlarms
where
    Self: Into<Batch> + Sized,
{
    //TODO use a type-safe Alarm, one with the typed Price
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, StableCurrency>) -> Result<()>;
}

pub trait AsAlarms {
    type Impl<'oref, OracleBase>: PriceAlarms
    where
        OracleBase: Currency;

    // TODO return `impl PriceAlarms` once swicth to Rust 1.75
    fn as_alarms<OracleBase>(&self) -> AlarmsStub<'_, OracleBase>
    where
        OracleBase: Currency;
}

impl AsAlarms for OracleRef {
    type Impl<'oref, OracleBase: Currency> = AlarmsStub<'oref, OracleBase>;

    fn as_alarms<OracleBase>(&self) -> AlarmsStub<'_, OracleBase>
    where
        OracleBase: Currency,
    {
        self.check_base::<OracleBase>();
        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
            _quote_currency: PhantomData::<OracleBase>,
        }
    }
}

pub struct AlarmsStub<'a, OracleBase> {
    oracle_ref: &'a OracleRef,
    _quote_currency: PhantomData<OracleBase>,
    batch: Batch,
}

impl<'a, OracleBase> AlarmsStub<'a, OracleBase> {
    fn addr(&self) -> &Addr {
        self.oracle_ref.addr()
    }
}

impl<'a, OracleBase> PriceAlarms for AlarmsStub<'a, OracleBase>
where
    OracleBase: Currency,
{
    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, StableCurrency>) -> Result<()> {
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
