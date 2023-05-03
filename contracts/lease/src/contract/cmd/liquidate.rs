use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use serde::Serialize;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use crate::{
    api::{LeaseCoin, LpnCoin},
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, Status},
};

use super::RepayResult;

pub(crate) type LiquidateResult = RepayResult;

pub(crate) struct Liquidate {
    asset: LeaseCoin,
    payment: LpnCoin,
    now: Timestamp,
}

impl Liquidate {
    pub fn new(asset: LeaseCoin, payment: LpnCoin, now: Timestamp) -> Self {
        Self {
            asset,
            payment,
            now,
        }
    }
}

impl WithLease for Liquidate {
    type Output = LiquidateResult;

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
        let receipt =
            lease.liquidate(self.asset.try_into()?, self.payment.try_into()?, self.now)?;

        let liquidation = match lease.liquidation_status(self.now)? {
            Status::No(zone) => {
                lease.reschedule(&self.now, &zone)?;
                None
            }
            Status::Liquidation(liquidation) => Some(liquidation.into()),
        };

        let IntoDTOResult {
            lease,
            batch: messages,
        } = lease.into_dto();

        Ok(Self::Output {
            lease,
            receipt: receipt.into(),
            messages,
            liquidation,
        })
    }
}
