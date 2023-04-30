use finance::{currency::Currency, liability::Zone};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use serde::Serialize;

use crate::{
    error::ContractError,
    lease::{with_lease::WithLease, IntoDTOResult, Lease, LiquidationDTO, Status},
};

pub(crate) struct Cmd {
    now: Timestamp,
}

pub(crate) enum CmdResult {
    NewAlarms {
        current_liability: Zone,
        alarms: Batch,
    },
    NeedLiquidation(LiquidationDTO),
}

impl Cmd {
    pub fn new(now: Timestamp) -> Self {
        Self { now }
    }
}

impl WithLease for Cmd {
    type Output = CmdResult;

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
        let status = lease.liquidation_status(self.now)?;
        let res = match status {
            Status::No(zone) => {
                lease.reschedule(&self.now, &zone)?;
                let IntoDTOResult {
                    batch: alarms,
                    lease: _,
                } = lease.into_dto();
                CmdResult::NewAlarms {
                    alarms,
                    current_liability: zone,
                }
            }
            Status::Liquidation(liquidation) => CmdResult::NeedLiquidation(liquidation.into()),
        };
        Ok(res)
    }
}
