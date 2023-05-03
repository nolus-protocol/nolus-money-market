use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO, Status},
    loan::RepayReceipt,
};

use super::liquidation_status::LiquidationDTO;

pub(crate) struct Repay {
    payment: LpnCoin,
    now: Timestamp,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

impl Repay {
    pub fn new(
        payment: LpnCoin,
        now: Timestamp,
        time_alarms: TimeAlarmsRef,
        price_alarms: OracleRef,
    ) -> Self {
        Self {
            payment,
            now,
            time_alarms,
            price_alarms,
        }
    }
}

pub(crate) struct RepayResult {
    pub lease: LeaseDTO,
    pub receipt: ReceiptDTO,
    pub messages: Batch,
    pub liquidation: Option<LiquidationDTO>,
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

    fn exec<Lpn, Asset, Lpp, Profit, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Profit, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let payment = self.payment.try_into()?;

        let receipt = lease.repay(payment, self.now)?;

        let (liquidation, time_alarms) = match lease.liquidation_status(self.now)? {
            Status::No(zone) => {
                let alarms_batch =
                    lease.reschedule(&self.now, &zone, self.time_alarms, self.price_alarms)?;
                (None, alarms_batch.time_alarms_ref)
            }
            Status::Liquidation(liquidation) => (Some(liquidation.into()), self.time_alarms),
        };

        let IntoDTOResult {
            lease,
            batch: messages,
        } = lease.into_dto(time_alarms);

        Ok(RepayResult {
            lease,
            receipt: receipt.into(),
            messages,
            liquidation,
        })
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
