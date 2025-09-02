use oracle_platform::OracleRef;

use currency::{Currency, CurrencyDef, Group, MemberOf};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Addr, StdError as CwError, wasm_execute};

use crate::api::alarms::{Alarm, Error, ExecuteMsg, Result};

pub trait PriceAlarms<AlarmCurrencies>
where
    AlarmCurrencies: Group,
    Self: Into<Batch> + Sized,
{
    type BaseC: CurrencyDef;
    // BaseC::Group: MemberOf<Self::BaseG>;
    type BaseG: Group;

    fn add_alarm(&mut self, alarm: Alarm<AlarmCurrencies, Self::BaseC, Self::BaseG>) -> Result<()>
    where
        <Self::BaseC as CurrencyDef>::Group:
            MemberOf<Self::BaseG> + MemberOf<AlarmCurrencies::TopG>;
}

pub trait AsAlarms<OracleBase, OracleBaseG>
where
    OracleBase: CurrencyDef + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn as_alarms<AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, BaseC = OracleBase, BaseG = OracleBaseG>
    where
        AlarmCurrencies: Group,
        OracleBase::Group: MemberOf<AlarmCurrencies::TopG>;
}

impl<OracleBase, OracleBaseG> AsAlarms<OracleBase, OracleBaseG>
    for OracleRef<OracleBase, OracleBaseG>
where
    OracleBase: CurrencyDef,
    OracleBase::Group: MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn as_alarms<AlarmCurrencies>(
        &self,
    ) -> impl PriceAlarms<AlarmCurrencies, BaseC = OracleBase, BaseG = OracleBaseG>
    where
        AlarmCurrencies: Group,
        OracleBase::Group: MemberOf<AlarmCurrencies::TopG>,
    {
        AlarmsStub {
            oracle_ref: self,
            batch: Batch::default(),
        }
    }
}

struct AlarmsStub<'a, OracleBase, OracleBaseG>
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    oracle_ref: &'a OracleRef<OracleBase, OracleBaseG>,
    batch: Batch,
}

impl<OracleBase, OracleBaseG> AlarmsStub<'_, OracleBase, OracleBaseG>
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn addr(&self) -> &Addr {
        self.oracle_ref.addr()
    }
}

impl<AlarmCurrencies, OracleBase, OracleBaseG> PriceAlarms<AlarmCurrencies>
    for AlarmsStub<'_, OracleBase, OracleBaseG>
where
    AlarmCurrencies: Group,
    OracleBase: CurrencyDef,
    OracleBase::Group: MemberOf<OracleBaseG> + MemberOf<AlarmCurrencies::TopG>,
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
            .map_err(|error: CwError| Error::StubAddAlarm(error.to_string()))?,
        );

        Ok(())
    }
}

impl<OracleBase, OracleBaseG> From<AlarmsStub<'_, OracleBase, OracleBaseG>> for Batch
where
    OracleBase: Currency + MemberOf<OracleBaseG>,
    OracleBaseG: Group,
{
    fn from(stub: AlarmsStub<'_, OracleBase, OracleBaseG>) -> Self {
        stub.batch
    }
}
