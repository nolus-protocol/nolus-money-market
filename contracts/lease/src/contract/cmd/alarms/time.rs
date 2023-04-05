use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::response::response_with_messages;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Env, Timestamp};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{with_lease::WithLease, Lease, OnAlarmResult},
};

use super::AlarmResult;

pub struct TimeAlarm<'a> {
    env: &'a Env,
    sender: &'a Addr,
    now: Timestamp,
}

impl<'a> TimeAlarm<'a> {
    pub fn new(env: &'a Env, sender: &'a Addr, now: Timestamp) -> Self {
        Self { env, sender, now }
    }
}

impl<'a> WithLease for TimeAlarm<'a> {
    type Output = AlarmResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        if !lease.sent_by_time_alarms(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        let OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        } = lease.on_time_alarm(self.now)?;

        response_with_messages(
            &self.env.contract.address,
            super::emit_events(self.env, &liquidation_status, batch),
        )
        .map(|response| AlarmResult {
            response,
            lease_dto,
        })
        .map_err(Into::into)
    }
}
