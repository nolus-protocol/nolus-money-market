use std::result::Result as StdResult;

use serde::{Deserialize, Serialize};

use platform::{batch::Batch, contract};
use sdk::cosmwasm_std::{wasm_execute, Addr, QuerierWrapper, Timestamp};

use crate::{msg::ExecuteMsg, ContractError};

pub type Result<T> = StdResult<T, ContractError>;

pub trait TimeAlarms
where
    Self: Into<Batch>,
{
    fn add_alarm(&mut self, time: Timestamp) -> Result<()>;
}

pub trait WithTimeAlarms {
    type Output;
    type Error;

    fn exec<TA>(self, time_alarms: TA) -> StdResult<Self::Output, Self::Error>
    where
        TA: TimeAlarms;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TimeAlarmsRef {
    addr: Addr,
}

impl TimeAlarmsRef {
    pub fn new(addr: Addr, querier: QuerierWrapper<'_>) -> Result<Self> {
        contract::validate_addr(querier, &addr)?;

        Ok(Self { addr })
    }

    pub fn owned_by(&self, addr: &Addr) -> bool {
        self.addr == addr
    }

    pub fn setup_alarm(&self, when: Timestamp) -> Result<Batch> {
        let mut stub = self.as_stub();
        stub.add_alarm(when)?;
        Ok(stub.into())
    }

    /// It would be overengineering to hide the `TimeAlarms` implementation
    fn as_stub(&self) -> TimeAlarmsStub<'_> {
        TimeAlarmsStub {
            time_alarms_ref: self,
            batch: Default::default(),
        }
    }
}

#[cfg(feature = "testing")]
impl TimeAlarmsRef {
    pub fn unchecked<A>(addr: A) -> Self
    where
        A: Into<String>,
    {
        Self {
            addr: Addr::unchecked(addr),
        }
    }
}

struct TimeAlarmsStub<'a> {
    time_alarms_ref: &'a TimeAlarmsRef,
    batch: Batch,
}

impl TimeAlarmsStub<'_> {
    fn addr(&self) -> &Addr {
        &self.time_alarms_ref.addr
    }
}

impl TimeAlarms for TimeAlarmsStub<'_> {
    fn add_alarm(&mut self, time: Timestamp) -> Result<()> {
        self.batch.schedule_execute_no_reply(wasm_execute(
            self.addr().clone(),
            &ExecuteMsg::AddAlarm { time },
            vec![],
        )?);

        Ok(())
    }
}

impl<'a> From<TimeAlarmsStub<'a>> for Batch {
    fn from(stub: TimeAlarmsStub<'a>) -> Self {
        stub.batch
    }
}
