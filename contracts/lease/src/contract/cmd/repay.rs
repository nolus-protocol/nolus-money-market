use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{
    batch::{Batch},
};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Env;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    api::LpnCoin,
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LeaseDTO, Status},
    loan::RepayReceipt,
};

use super::liquidation_status::LiquidationDTO;

pub(crate) struct Repay<'a> {
    payment: LpnCoin,
    env: &'a Env,
}

impl<'a> Repay<'a> {
    pub fn new(payment: LpnCoin, env: &'a Env) -> Self {
        Self { payment, env }
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

impl<'a> WithLease for Repay<'a> {
    type Output = RepayResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let now = self.env.block.time;
        let payment = self.payment.try_into()?;

        let receipt = lease.repay(payment, now)?;

        let liquidation = match lease.liquidation_status(now)? {
            Status::No(zone) => {
                lease.reschedule(&now, &zone)?;
                None
            }
            Status::Liquidation(liquidation) => Some(liquidation.into()),
        };

        let IntoDTOResult {
            lease,
            batch: messages,
        } = lease.into_dto();

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
