use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::BankAccount;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Addr;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{IntoDTOResult, Lease, WithLease},
};

pub struct Close<'a, Bank> {
    sender: &'a Addr,
    account: Bank,
}

impl<'a, Bank> Close<'a, Bank> {
    pub fn new(sender: &'a Addr, account: Bank) -> Self {
        Self { sender, account }
    }
}

impl<'a, Bank> WithLease for Close<'a, Bank>
where
    Bank: BankAccount,
{
    type Output = IntoDTOResult;

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

        Ok(result)
    }
}
