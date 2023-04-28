use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{Lease, LeaseDTO},
    loan::RepayReceipt,
};

use super::{IntoDTOResult, Status};

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
        mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<Result<Lpn>> {
        let receipt = self.no_reschedule_repay(payment, now)?;

        //TODO move the liquidation status checks and alarm rescheduling up to the cmd-s
        match self.liquidation_status(now)? {
            Status::No(zone) => self.reschedule(&now, &zone)?,
            _ => todo!(),
        }

        let IntoDTOResult { lease, batch } = self.into_dto();

        Ok(Result {
            batch,
            lease,
            receipt,
        })
    }

    pub(super) fn no_reschedule_repay(
        &mut self,
        payment: Coin<Lpn>,
        now: Timestamp,
    ) -> ContractResult<RepayReceipt<Lpn>> {
        self.loan.repay(payment, now, self.addr.clone())
    }
}

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease: LeaseDTO,
    pub receipt: RepayReceipt<Lpn>,
}
