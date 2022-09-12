use std::result::Result as StdResult;

use cosmwasm_std::{wasm_execute, Addr, Timestamp};
use serde::{Deserialize, Serialize};

use platform::batch::Batch;

use crate::{msg::ExecuteMsg, ContractError};

pub type Result<T> = StdResult<T, ContractError>;

pub struct TimeAlarmsBatch {
    pub time_alarms_ref: TimeAlarmsRef,
    pub batch: Batch,
}

pub trait TimeAlarms
where
    Self: Into<TimeAlarmsBatch>,
{
    fn owned_by(&self, addr: &Addr) -> bool;

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
pub struct TimeAlarmsRef {
    addr: Addr,
}

impl TimeAlarmsRef {
    fn owned_by(&self, addr: &Addr) -> bool {
        &self.addr == addr
    }

    pub fn execute<Cmd>(self, cmd: Cmd) -> StdResult<Cmd::Output, Cmd::Error>
    where
        Cmd: WithTimeAlarms,
    {
        cmd.exec(self.into_stub())
    }

    fn into_stub(self) -> TimeAlarmsStub {
        TimeAlarmsStub {
            time_alarms_ref: self,
            batch: Default::default(),
        }
    }
}

impl From<Addr> for TimeAlarmsRef {
    fn from(addr: Addr) -> Self {
        Self { addr }
    }
}

impl From<TimeAlarmsRef> for Addr {
    fn from(oracle_ref: TimeAlarmsRef) -> Self {
        oracle_ref.addr
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

struct TimeAlarmsStub {
    time_alarms_ref: TimeAlarmsRef,
    batch: Batch,
}

impl TimeAlarmsStub {
    fn addr(&self) -> &Addr {
        &self.time_alarms_ref.addr
    }
}

impl TimeAlarms for TimeAlarmsStub {
    fn owned_by(&self, addr: &Addr) -> bool {
        self.time_alarms_ref.owned_by(addr)
    }

    fn add_alarm(&mut self, time: Timestamp) -> Result<()> {
        self.batch.schedule_execute_no_reply(wasm_execute(
            self.addr().clone(),
            &ExecuteMsg::AddAlarm { time },
            vec![],
        )?);

        Ok(())
    }
}

impl From<TimeAlarmsStub> for TimeAlarmsBatch {
    fn from(stub: TimeAlarmsStub) -> Self {
        TimeAlarmsBatch {
            time_alarms_ref: stub.time_alarms_ref,
            batch: stub.batch,
        }
    }
}
