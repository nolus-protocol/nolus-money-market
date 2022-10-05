use cosmwasm_std::{Addr, QuerierWrapper};
use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use market_price_oracle::stub::Oracle as OracleTrait;
use platform::bank::FixedAddressSenderBuilder;
use profit::stub::Profit as ProfitTrait;
use serde::Serialize;
use time_alarms::stub::TimeAlarms as TimeAlarmsTrait;

use super::{factory::Factory, Lease, LeaseDTO};

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
    dto: LeaseDTO,
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
    let lpp = dto.loan.lpp().clone();

    lpp.execute(
        Factory::new(cmd, dto, addr, sender_builder, querier),
        querier,
    )
}
