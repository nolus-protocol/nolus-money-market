use cosmwasm_std::{to_binary, Binary, Timestamp};
use serde::Serialize;

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::BankAccount;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{Lease, WithLease},
};

pub struct LeaseState<Bank> {
    now: Timestamp,
    account: Bank,
}

impl<Bank> LeaseState<Bank> {
    pub fn new(now: Timestamp, account: Bank) -> Self {
        Self {
            now,
            account,
        }
    }
}

impl<Bank> WithLease for LeaseState<Bank>
where
    Bank: BankAccount,
{
    type Output = Binary;

    type Error = ContractError;

    fn exec<Lpn, Lpp, TimeAlarms, Oracle, Asset>(
        self,
        lease: Lease<Lpn, Lpp, TimeAlarms, Oracle, Asset>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency + Serialize,
    {
        let resp = lease.state(self.now, &self.account)?;
        to_binary(&resp).map_err(ContractError::from)
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        Err(ContractError::UnknownCurrency { symbol })
    }
}
