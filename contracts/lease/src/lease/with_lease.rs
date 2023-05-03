use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::QuerierWrapper;

use super::{
    with_lease_deps::{self, WithLeaseDeps},
    Lease, LeaseDTO,
};

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Asset, Lpp, Profit, Oracle>(
        self,
        lease: Lease<Lpn, Asset, Lpp, Profit, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize;
}

pub fn execute<Cmd>(
    lease_dto: LeaseDTO,
    cmd: Cmd,
    querier: &QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLease,
    finance::error::Error: Into<Cmd::Error>,
    timealarms::error::ContractError: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
    profit::error::ContractError: Into<Cmd::Error>,
{
    let asset = lease_dto.amount.ticker().clone();
    let lpp = lease_dto.loan.lpp().clone();
    let profit = lease_dto.loan.profit().clone();
    let oracle = lease_dto.oracle.clone();

    with_lease_deps::execute(
        Factory::new(cmd, lease_dto),
        &asset,
        lpp,
        profit,
        oracle,
        querier,
    )
}

struct Factory<Cmd> {
    cmd: Cmd,
    lease_dto: LeaseDTO,
}
impl<Cmd> Factory<Cmd> {
    fn new(cmd: Cmd, lease_dto: LeaseDTO) -> Self {
        Self { cmd, lease_dto }
    }
}

impl<Cmd> WithLeaseDeps for Factory<Cmd>
where
    Cmd: WithLease,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn exec<Lpn, Asset, Lpp, Profit, Oracle>(
        self,
        lpp: Lpp,
        profit: Profit,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        self.cmd.exec(Lease::<_, Asset, _, _, _>::from_dto(
            self.lease_dto,
            lpp,
            oracle,
            profit,
        ))
    }
}
