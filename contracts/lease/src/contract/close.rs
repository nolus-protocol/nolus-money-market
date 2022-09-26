use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::{
    bank::BankAccount,
    batch::{Emit, Emitter},
};
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    event::TYPE,
    lease::{Lease, WithLease},
};

pub struct Close<'a, Bank> {
    sender: &'a Addr,
    lease: Addr,
    account: Bank,
    now: Timestamp,
}

impl<'a, Bank> Close<'a, Bank> {
    pub fn new(sender: &'a Addr, lease: Addr, account: Bank, now: Timestamp) -> Self {
        Self {
            sender,
            lease,
            account,
            now,
        }
    }
}

impl<'a, Bank> WithLease for Close<'a, Bank>
where
    Bank: BankAccount,
{
    type Output = Emitter;

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
        if !lease.owned_by(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        let result = lease.close(self.account)?;

        let emitter = result
            .into_emitter(TYPE::Close)
            .emit("id", self.lease)
            .emit_timestamp("at", &self.now);

        Ok(emitter)
    }
}
