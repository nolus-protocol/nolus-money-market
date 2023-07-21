use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use serde::Serialize;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO},
};

use super::ReceiptDTO;

pub(crate) struct LiquidateResult {
    pub lease: LeaseDTO,
    pub receipt: ReceiptDTO,
    pub messages: Batch,
}

pub(crate) struct Liquidate {
    payment: LpnCoin,
    now: Timestamp,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
}

impl Liquidate {
    pub fn new(
        payment: LpnCoin,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
    ) -> Self {
        Self {
            payment,
            now,
            profit,
            time_alarms,
        }
    }
}

impl WithLease for Liquidate {
    type Output = LiquidateResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency + Serialize,
    {
        let mut profit = self.profit.as_stub();
        let receipt = lease.repay(self.payment.try_into()?, self.now, &mut profit)?;

        if !receipt.close() {
            return Err(ContractError::InsufficientLiquidation()); //issue #92
        }

        lease.try_into_dto(self.profit, self.time_alarms).map(
            |IntoDTOResult {
                 lease,
                 batch: messages,
             }| Self::Output {
                lease,
                receipt: receipt.into(),
                messages: messages.merge(profit.into()),
            },
        )
    }
}
