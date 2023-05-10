use serde::Serialize;

use finance::currency::Currency;
use lpp::stub::loan::LppLoan as LppLoanTrait;
use oracle::stub::Oracle as OracleTrait;
use sdk::cosmwasm_std::QuerierWrapper;

use super::{
    with_lease_deps::{self, WithLeaseDeps},
    Lease, LeaseDTO,
};

pub trait WithLease {
    type Output;
    type Error;

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lease: Lease<Lpn, Asset, LppLoan, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Asset: Currency + Serialize,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>;
}

pub fn execute<Cmd>(
    lease_dto: LeaseDTO,
    cmd: Cmd,
    querier: &QuerierWrapper<'_>,
) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLease,
    Cmd::Error: From<lpp::error::ContractError>,
    finance::error::Error: Into<Cmd::Error>,
    timealarms::error::ContractError: Into<Cmd::Error>,
    oracle::error::ContractError: Into<Cmd::Error>,
{
    let lease = lease_dto.addr.clone();
    let asset = lease_dto.amount.ticker().clone();
    let lpp = lease_dto.loan.lpp().clone();
    let oracle = lease_dto.oracle.clone();

    with_lease_deps::execute(
        Factory::new(cmd, lease_dto),
        lease,
        &asset,
        lpp,
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

    fn exec<Lpn, Asset, LppLoan, Oracle>(
        self,
        lpp_loan: LppLoan,
        oracle: Oracle,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        LppLoan: LppLoanTrait<Lpn>,
        Oracle: OracleTrait<Lpn>,
        Asset: Currency + Serialize,
    {
        self.cmd.exec(Lease::<_, Asset, _, _>::from_dto(
            self.lease_dto,
            lpp_loan,
            oracle,
        ))
    }
}
