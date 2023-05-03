use serde::{Deserialize, Serialize};

use finance::{currency::Currency, liability::Zone};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LeaseCoin,
    error::ContractError,
    lease::{with_lease::WithLease, Cause, Lease, LeaseDTO, Liquidation, Status},
};

pub(crate) struct Cmd {
    now: Timestamp,
    time_alarms: TimeAlarmsRef,
    price_alarms: OracleRef,
}

pub(crate) enum CmdResult {
    NewAlarms {
        current_liability: Zone,
        alarms: Batch,
    },
    NeedLiquidation(LiquidationDTO),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum LiquidationDTO {
    Partial { amount: LeaseCoin, cause: Cause },
    Full(Cause),
}

impl LiquidationDTO {
    pub(crate) fn amount<'a>(&'a self, lease: &'a LeaseDTO) -> &LeaseCoin {
        match self {
            Self::Partial { amount, cause: _ } => amount,
            Self::Full(_) => &lease.amount,
        }
    }

    pub(crate) fn cause(&self) -> &Cause {
        match self {
            Self::Partial { amount: _, cause } => cause,
            Self::Full(cause) => cause,
        }
    }
}

impl<Asset> From<Liquidation<Asset>> for LiquidationDTO
where
    Asset: Currency,
{
    fn from(value: Liquidation<Asset>) -> Self {
        match value {
            Liquidation::Partial { amount, cause } => Self::Partial {
                amount: amount.into(),
                cause,
            },
            Liquidation::Full(cause) => Self::Full(cause),
        }
    }
}

impl Cmd {
    pub fn new(now: Timestamp, time_alarms: TimeAlarmsRef, price_alarms: OracleRef) -> Self {
        Self {
            now,
            time_alarms,
            price_alarms,
        }
    }
}

impl WithLease for Cmd {
    type Output = CmdResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let status = lease.liquidation_status(self.now)?;
        let res = match status {
            Status::No(zone) => {
                let alarms =
                    lease.reschedule(&self.now, &zone, self.time_alarms, self.price_alarms)?;
                CmdResult::NewAlarms {
                    alarms: alarms.batch,
                    current_liability: zone,
                }
            }
            Status::Liquidation(liquidation) => CmdResult::NeedLiquidation(liquidation.into()),
        };
        Ok(res)
    }
}
