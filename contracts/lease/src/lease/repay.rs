use std::any::TypeId;

use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::price::{total, Price};
use finance::{coin::Coin, currency::Currency};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    error::ContractResult,
    lease::{Lease, LeaseDTO},
    loan::RepayReceipt,
};

impl<Lpn, Lpp, TimeAlarms, Oracle, Asset> Lease<Lpn, Lpp, TimeAlarms, Oracle, Asset>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    TimeAlarms: TimeAlarmsTrait,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency + Serialize,
{
    pub(crate) fn repay(
        mut self,
        lease_amount: Coin<Asset>,
        payment: Coin<Asset>,
        now: Timestamp,
        lease: Addr,
    ) -> ContractResult<Result<Lpn>> {
        // TODO perform swap
        assert_eq!(TypeId::of::<Asset>(), TypeId::of::<Lpn>());
        let payment_lpn = total(payment, Price::identity());

        let receipt = self.loan.repay(payment_lpn, now, lease.clone())?;

        self.reschedule_on_repay(lease, lease_amount, &now)?;

        let (lease_dto, batch) = self.into_dto();

        Ok(Result {
            batch,
            lease_dto,
            receipt,
        })
    }
}

pub(crate) struct Result<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub receipt: RepayReceipt<Lpn>,
}
