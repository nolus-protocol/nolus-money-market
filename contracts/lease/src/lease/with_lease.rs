use cosmwasm_std::{Addr, QuerierWrapper};
use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use serde::Serialize;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use super::{
    with_lease_deps::{self, WithLeaseDeps},
    Lease, LeaseDTO,
};

pub trait WithLease {
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

pub fn execute<Cmd>(
    lease_dto: LeaseDTO,
    cmd: Cmd,
    addr: &Addr,
    querier: &QuerierWrapper,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    time_alarms::error::ContractError: Into<Cmd::Error>,
    market_price_oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    let asset = lease_dto.currency.clone();
    let lpp = lease_dto.loan.lpp().clone();
    let profit = lease_dto.loan.profit().clone();
    let alarms = lease_dto.time_alarms.clone();
    let oracle = lease_dto.oracle.clone();

    with_lease_deps::execute(
        Factory::new(cmd, lease_dto, addr),
        &asset,
        lpp,
        profit,
        alarms,
        oracle,
        querier,
    )
}

struct Factory<'r, Cmd> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
    lease_addr: &'r Addr,
}
impl<'r, Cmd> Factory<'r, Cmd> {
    fn new(cmd: Cmd, lease_dto: LeaseDTO, lease_addr: &'r Addr) -> Self {
        Self {
            cmd,
            lease_dto,
            lease_addr,
        }
    }
}

impl<'r, Cmd> WithLeaseDeps for Factory<'r, Cmd>
where
    Cmd: WithLease,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        time_alarms: TimeAlarms,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        self.cmd.exec(Lease::<_, Asset, _, _, _, _>::from_dto(
            self.lease_dto,
            self.lease_addr,
            lpp,
            time_alarms,
            oracle,
            profit,
        ))
    }
}
