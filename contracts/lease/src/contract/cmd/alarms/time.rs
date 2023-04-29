use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::message::Response as MessageResponse;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::{Addr, Timestamp};
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

pub struct TimeAlarm<'a> {
    sender: &'a Addr,
    now: Timestamp,
}

impl<'a> TimeAlarm<'a> {
    pub fn new(sender: &'a Addr, now: Timestamp) -> Self {
        Self { sender, now }
    }
}

impl<'a> WithLease for TimeAlarm<'a> {
    type Output = MessageResponse;

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

        super::handle(self.now, lease)
    }
}
