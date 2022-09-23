use cosmwasm_std::{Addr, QuerierWrapper};
use serde::Serialize;

use finance::{currency::Currency, error::Error as FinanceError};
use lpp::{error::ContractError as LppError, stub::lender::LppLender as LppLenderTrait};
use market_price_oracle::{error::ContractError as OracleError, stub::Oracle as OracleTrait};
use profit::{error::ContractError as ProfitError, stub::Profit as ProfitTrait};
use time_alarms::{error::ContractError as TimeAlarmsError, stub::TimeAlarms as TimeAlarmsTrait};

use crate::{
    error::ContractError,
    lease::factory::Factory,
    lease::{Lease, LeaseDTO},
};

pub trait WithLease
where
    ContractError: Into<Self::Error>,
{
    type Output;
    type Error;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize;
}

pub fn execute<L>(
    dto: LeaseDTO,
    cmd: L,
    addr: &Addr,
    querier: &QuerierWrapper,
) -> Result<L::Output, L::Error>
where
    L: WithLease,
    ContractError: Into<L::Error>,
    FinanceError: Into<L::Error>,
    LppError: Into<L::Error>,
    TimeAlarmsError: Into<L::Error>,
    OracleError: Into<L::Error>,
    ProfitError: Into<L::Error>,
{
    let lpp = dto.loan.lpp().clone();

    lpp.execute(Factory::new(cmd, dto, addr, querier), querier)
}
