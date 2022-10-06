use cosmwasm_std::{Addr, QuerierWrapper};
use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef};
use platform::bank::FixedAddressSenderBuilder;
use profit::stub::Profit as ProfitTrait;
use serde::Serialize;
use time_alarms::stub::{TimeAlarms as TimeAlarmsTrait, TimeAlarmsRef};

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

pub fn execute<Cmd, SenderBuilder>(
    lease_dto: LeaseDTO,
    cmd: Cmd,
    addr: &Addr,
    sender_builder: SenderBuilder,
    querier: &QuerierWrapper,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    time_alarms::error::ContractError: Into<Cmd::Error>,
    market_price_oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
    SenderBuilder: FixedAddressSenderBuilder,
{
    let asset = lease_dto.currency.clone();
    let lpp = lease_dto.loan.lpp().clone();
    let profit = lease_dto.loan.profit().clone();
    let alarms = TimeAlarmsRef::try_from(lease_dto.time_alarms.clone())
        .expect("Time Alarms is not deployed, or wrong address is passed!");
    let oracle = OracleRef::try_from(lease_dto.oracle.clone(), querier)
        .expect("Market Price Oracle is not deployed, or wrong address is passed!");

    with_lease_deps::execute(
        Factory::new(cmd, lease_dto, addr),
        &asset,
        lpp,
        profit,
        alarms,
        oracle,
        sender_builder,
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
