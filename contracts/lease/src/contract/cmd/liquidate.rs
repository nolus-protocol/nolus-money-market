use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use serde::Serialize;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseCoin, LpnCoin},
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, Reschedule, Status},
};

use super::RepayResult;

pub(crate) type LiquidateResult = RepayResult;

pub(crate) struct Liquidate {
    asset: LeaseCoin,
    payment: LpnCoin,
    now: Timestamp,
    time_alarms: TimeAlarmsRef,
}

impl Liquidate {
    pub fn new(
        asset: LeaseCoin,
        payment: LpnCoin,
        now: Timestamp,
        time_alarms: TimeAlarmsRef,
    ) -> Self {
        Self {
            asset,
            payment,
            now,
            time_alarms,
        }
    }
}

impl WithLease for Liquidate {
    type Output = LiquidateResult;

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
        let receipt =
            lease.liquidate(self.asset.try_into()?, self.payment.try_into()?, self.now)?;

        let (liquidation, time_alarms) = match lease.liquidation_status(self.now)? {
            Status::No(zone) => {
                let alarms = self
                    .time_alarms
                    .execute(Reschedule(&mut lease, &self.now, &zone))?;
                (None, alarms.time_alarms_ref)
            }
            Status::Liquidation(liquidation) => (Some(liquidation.into()), self.time_alarms),
        };

        let IntoDTOResult {
            lease,
            batch: messages,
        } = lease.into_dto(time_alarms);

        Ok(Self::Output {
            lease,
            receipt: receipt.into(),
            messages,
            liquidation,
        })
    }
}
