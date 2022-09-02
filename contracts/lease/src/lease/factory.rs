use cosmwasm_std::{Api, QuerierWrapper};
use serde::Serialize;

use finance::currency::{Currency, SymbolOwned};
use lpp::stub::{Lpp as LppTrait, WithLpp};
use market_price_oracle::stub::{Oracle as OracleTrait, OracleRef, WithOracle};

use super::{dto::LeaseDTO, Lease, WithLease};

pub struct Factory<'r, L> {
    cmd: L,
    lease_dto: LeaseDTO,
    api: &'r dyn Api,
    querier: &'r QuerierWrapper<'r>,
}

impl<'r, L> Factory<'r, L> {
    pub fn new(
        cmd: L,
        lease_dto: LeaseDTO,
        api: &'r dyn Api,
        querier: &'r QuerierWrapper<'r>,
    ) -> Self {
        Self {
            cmd,
            lease_dto,
            api,
            querier,
        }
    }
}

impl<'r, L, O, E> WithLpp for Factory<'r, L>
where
    L: WithLease<Output = O, Error = E>,
{
    type Output = O;
    type Error = E;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppTrait<Lpn>,
    {
        let oracle = OracleRef::try_from(self.lease_dto.oracle.to_string(), self.api, self.querier);

        oracle.expect("Constructing OracleRef failed!").execute(
            FactoryStage2 {
                cmd: self.cmd,
                lease_dto: self.lease_dto,
                lpp,
            },
            self.querier,
        )
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(symbol)
    }
}

struct FactoryStage2<L, Lpp> {
    cmd: L,
    lease_dto: LeaseDTO,
    lpp: Lpp,
}

impl<L, Lpn, Lpp> WithOracle<Lpn> for FactoryStage2<L, Lpp>
where
    L: WithLease,
    Lpp: LppTrait<Lpn>,
    Lpn: Currency + Serialize,
{
    type Output = L::Output;
    type Error = L::Error;

    fn exec<Oracle>(self, oracle: Oracle) -> Result<Self::Output, Self::Error>
    where
        Oracle: OracleTrait<Lpn>,
    {
        self.cmd
            .exec(Lease::from_dto(self.lease_dto, self.lpp, oracle))
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(symbol)
    }
}
