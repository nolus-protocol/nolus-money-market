use serde::{Deserialize, Serialize};

use finance::{currency::Currency, liability::Zone};
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LeaseCoin,
    error::{ContractError, ContractResult},
    lease::{with_lease::WithLease, Cause, Lease, LeaseDTO, Liquidation, Status},
};

pub(crate) fn status_and_schedule<Lpn, Asset, Lpp, Oracle>(
    lease: &Lease<Lpn, Asset, Lpp, Oracle>,
    when: Timestamp,
    time_alarms: &TimeAlarmsRef,
    price_alarms: &OracleRef,
) -> ContractResult<CmdResult>
where
    Lpn: Currency + Serialize,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency + Serialize,
{
    let status = lease.liquidation_status(when)?;
    Ok(match status {
        Status::NoDebt => CmdResult::NoDebt,
        Status::No(zone) => CmdResult::NewAlarms {
            alarms: lease.reschedule(&when, &zone, time_alarms, price_alarms)?,
            current_liability: zone,
        },
        Status::Liquidation(liquidation) => CmdResult::NeedLiquidation(liquidation.into()),
    })
}

pub(crate) struct Cmd<'a> {
    now: Timestamp,
    time_alarms: &'a TimeAlarmsRef,
    price_alarms: &'a OracleRef,
}

pub(crate) enum CmdResult {
    NoDebt,
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

impl<'a> Cmd<'a> {
    pub fn new(
        now: Timestamp,
        time_alarms: &'a TimeAlarmsRef,
        price_alarms: &'a OracleRef,
    ) -> Self {
        Self {
            now,
            time_alarms,
            price_alarms,
        }
    }
}

impl<'a> WithLease for Cmd<'a> {
    type Output = CmdResult;

    type Error = ContractError;

    fn exec<Lpn, Asset, Loan, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Loan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency + Serialize,
    {
        status_and_schedule(&lease, self.now, self.time_alarms, self.price_alarms)
    }
}
