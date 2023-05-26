use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use profit::stub::ProfitRef;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO},
    loan::RepayReceipt,
};

use super::{liquidation_status, LiquidationStatus};

pub(crate) struct Repay {
    payment: LpnCoin,
    now: Timestamp,
    profit: ProfitRef,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

impl Repay {
    pub fn new(
        payment: LpnCoin,
        now: Timestamp,
        profit: ProfitRef,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
    ) -> Self {
        Self {
            payment,
            now,
            profit,
            time_alarms,
            price_alarms,
        }
    }
}

pub(crate) struct RepayResult {
    pub lease: LeaseDTO,
    pub receipt: ReceiptDTO,
    pub messages: Batch,
    pub liquidation: LiquidationStatus,
}

pub(crate) struct ReceiptDTO {
    pub total: LpnCoin,
    pub previous_margin_paid: LpnCoin,
    pub current_margin_paid: LpnCoin,
    pub previous_interest_paid: LpnCoin,
    pub current_interest_paid: LpnCoin,
    pub principal_paid: LpnCoin,
    pub change: LpnCoin,
    pub close: bool,
}

impl WithLease for Repay {
    type Output = RepayResult;

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
        let payment = self.payment.try_into()?;
        let mut profit = self.profit.as_stub();

        let receipt = lease.repay(payment, self.now, &mut profit)?;

        let liquidation = liquidation_status::status_and_schedule(
            &lease,
            self.now,
            &self.time_alarms,
            &self.price_alarms,
        )?;

        lease.try_into_dto(self.profit, self.time_alarms).map(
            |IntoDTOResult {
                 lease,
                 batch: messages,
             }| {
                RepayResult {
                    lease,
                    receipt: receipt.into(),
                    messages: messages.merge(profit.into()),
                    liquidation,
                }
            },
        )
    }
}

impl<Lpn> From<RepayReceipt<Lpn>> for ReceiptDTO
where
    Lpn: Currency,
{
    fn from(value: RepayReceipt<Lpn>) -> Self {
        Self {
            total: value.total().into(),
            previous_margin_paid: value.previous_margin_paid().into(),
            current_margin_paid: value.current_margin_paid().into(),
            previous_interest_paid: value.previous_interest_paid().into(),
            current_interest_paid: value.current_interest_paid().into(),
            principal_paid: value.principal_paid().into(),
            change: value.change().into(),
            close: value.close(),
        }
    }
}
