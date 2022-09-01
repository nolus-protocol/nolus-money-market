use cosmwasm_std::{Addr, Timestamp};
use serde::Serialize;

use finance::{coin::Coin, currency::Currency};
use lpp::stub::Lpp as LppTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;

use crate::{
    error::ContractResult,
    lease::{Lease, LeaseDTO},
    loan::Receipt,
};

impl<Lpn, Lpp, Oracle> Lease<Lpn, Lpp, Oracle>
where
    Lpn: Currency + Serialize,
    Lpp: LppTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
{
    pub(crate) fn repay(
        mut self,
        lease_amount: Coin<Lpn>,
        payment: Coin<Lpn>,
        now: Timestamp,
        lease: Addr,
    ) -> ContractResult<Result<Lpn>> {
        assert_eq!(self.currency, Lpn::SYMBOL);

        let receipt = self.loan.repay(payment, now, lease.clone())?;

        self.reschedule_on_repay(lease, lease_amount, &now)?;

        let (lease_dto, lpp, oracle) = self.into_dto();

        let batch = Into::<Batch>::into(lpp).merge(oracle.into());

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
    pub receipt: Receipt<Lpn>,
}
