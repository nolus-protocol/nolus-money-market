use cosmwasm_std::{Addr, Env, Timestamp};
use serde::Serialize;

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::BankAccount;
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    contract::alarms::{emit_events, AlarmResult},
    error::ContractError,
    lease::{Lease, OnAlarmResult, WithLease},
};

pub struct TimeAlarm<'a, B>
where
    B: BankAccount,
{
    env: &'a Env,
    sender: &'a Addr,
    account: B,
    now: Timestamp,
}

impl<'a, B> TimeAlarm<'a, B>
where
    B: BankAccount,
{
    pub fn new(env: &'a Env, sender: &'a Addr, account: B, now: Timestamp) -> Self {
        Self {
            env,
            sender,
            account,
            now,
        }
    }
}

impl<'a, B> WithLease for TimeAlarm<'a, B>
where
    B: BankAccount,
{
    type Output = AlarmResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        mut self,
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
            mut batch,
            lease_dto,
            liquidation_status,
        } = lease.on_time_alarm(self.now, &mut self.account)?;

        batch = batch.merge(self.account.into());

        Ok(AlarmResult {
            response: emit_events(self.env, &liquidation_status, batch),
            lease_dto,
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
