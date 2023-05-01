use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{bank::BankAccount, batch::Batch};
use profit::stub::Profit as ProfitTrait;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

pub struct Close<Bank> {
    lease_account: Bank,
}

impl<Bank> Close<Bank> {
    pub fn new(lease_account: Bank) -> Self {
        Self { lease_account }
    }
}

impl<Bank> WithLease for Close<Bank>
where
    Bank: BankAccount,
{
    type Output = Batch;

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
        lease.close(self.lease_account)
    }
}
