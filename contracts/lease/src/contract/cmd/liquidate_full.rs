use currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{bank::FixedAddressSender, batch::Batch};
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use serde::Serialize;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Lease},
};

use super::ReceiptDTO;

pub(crate) struct LiquidateResult {
    pub receipt: ReceiptDTO,
    pub messages: Batch,
}

impl LiquidateResult {
    fn new(receipt: ReceiptDTO, messages: Batch) -> Self {
        debug_assert!(
            receipt.close,
            "The full-liquidation payment should have repaid the total outstanding liability!"
        );
        Self { receipt, messages }
    }
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

        profit.send(receipt.change());

        lease
            .try_into_dto(self.profit, self.time_alarms)
            .map(|dto_result| {
                Self::Output::new(receipt.into(), dto_result.batch.merge(profit.into()))
            })
    }
}
