use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{error::ContractResult, lease::Lease, loan::RepayReceipt};

impl<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle> Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppLenderTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Profit: ProfitTrait,
    Asset: Currency + Serialize,
{
    pub(crate) fn repay(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.loan.repay(payment, now, self.addr.clone())
    }
}
