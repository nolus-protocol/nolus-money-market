use serde::{Deserialize, Serialize};

use currency::Currency;
use finance::liability::Zone;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle_platform::{Oracle as OracleTrait, OracleRef};
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::LeaseCoin,
    error::{ContractError, ContractResult},
    lease::{with_lease::WithLease, Lease as LeaseDO},
    position::{Cause, Liquidation, Status},
};

pub(crate) fn status_and_schedule<Lpn, Asset, Lpp, Oracle>(
    lease: &LeaseDO<Lpn, Asset, Lpp, Oracle>,
    when: Timestamp,
    time_alarms: &TimeAlarmsRef,
    price_alarms: &OracleRef,
) -> ContractResult<CmdResult>
where
    Lpn: Currency,
    Lpp: LppLoanTrait<Lpn>,
    Oracle: OracleTrait<Lpn>,
    Asset: Currency,
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
    Partial(PartialLiquidationDTO),
    Full(FullLiquidationDTO),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PartialLiquidationDTO {
    pub amount: LeaseCoin,
    pub cause: Cause,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct FullLiquidationDTO {
    pub cause: Cause,
}

impl<Asset> From<Liquidation<Asset>> for LiquidationDTO
where
    Asset: Currency,
{
    fn from(value: Liquidation<Asset>) -> Self {
        match value {
            Liquidation::Partial { amount, cause } => Self::Partial(PartialLiquidationDTO {
                amount: amount.into(),
                cause,
            }),
            Liquidation::Full(cause) => Self::Full(FullLiquidationDTO { cause }),
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
        lease: LeaseDO<Lpn, Asset, Loan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency,
        Loan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency,
    {
        status_and_schedule(&lease, self.now, self.time_alarms, self.price_alarms)
    }
}
