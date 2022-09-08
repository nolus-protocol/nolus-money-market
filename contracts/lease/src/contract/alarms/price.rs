use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{
    currency::{Currency, SymbolOwned},
    price::PriceDTO,
};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::BankAccountView;
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
    sender: &'a Addr,
    lease: Addr,
    account: B,
    now: Timestamp,
    price: PriceDTO,
}

impl<'a, B> PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    pub fn new(sender: &'a Addr, lease: Addr, account: B, now: Timestamp, price: PriceDTO) -> Self {
        Self {
            sender,
            lease,
            account,
            now,
            price,
        }
    }
}

impl<'a, B> WithLease for PriceAlarm<'a, B>
where
    B: BankAccountView,
{
    type Output = AlarmResult;

    type Error = ContractError;

    fn exec<Lpn, Lpp, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Lpp, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
    {
        if !lease.sent_by_oracle(self.sender) {
            return Err(Self::Error::Unauthorized {});
        }

        let OnAlarmResult {
            batch,
            lease_dto,
            liquidation_status,
        } = lease.on_price_alarm(
            self.now,
            &self.account,
            self.lease.clone(),
        )?;

        Ok(AlarmResult {
            response: emit_events(&liquidation_status, batch),
            lease_dto,
        })
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
