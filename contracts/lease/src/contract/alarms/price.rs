use cosmwasm_std::{Addr, Env, Timestamp};
use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::BankAccountView;
use profit::stub::Profit as ProfitTrait;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    contract::alarms::{emit_events, AlarmResult},
    error::ContractError,
    lease::{Lease, OnAlarmResult, WithLease},
};

pub struct PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    env: &'a Env,
    sender: &'a Addr,
    account: B,
    now: Timestamp,
}

impl<'a, B> PriceAlarm<'a, B>
where
    B: BankAccountView,
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

impl<'a, B> WithLease for PriceAlarm<'a, B>
where
    B: BankAccountView,
{
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
        if !lease.sent_by_oracle(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        let OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        } = lease.on_price_alarm(self.now, &self.account)?;

        Ok(AlarmResult {
            response: emit_events(self.env, &liquidation_status, batch),
            lease_dto,
        })
    }
}
