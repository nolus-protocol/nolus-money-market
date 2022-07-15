use finance::currency::{Currency, SymbolOwned};
use lpp::stub::{Lpp as LppTrait, WithLpp};
use serde::Serialize;

use super::{dto::LeaseDTO, Lease, WithLease};

pub struct Factory<L>
{
    cmd: L,
    lease_dto: LeaseDTO,
}

impl<L> Factory<L>
{
    pub fn new(cmd: L, lease_dto: LeaseDTO) -> Self {
        Self { cmd, lease_dto }
    }
}

impl<L, O, E> WithLpp for Factory<L>
where
    L: WithLease<Output = O, Error = E>,
{
    type Output = O;
    type Error = E;

    fn exec<Lpn, Lpp>(self, lpp: Lpp) -> Result<Self::Output, Self::Error>
    where
        Lpp: LppTrait<Lpn>,
        Lpn: Currency + Serialize,
    {
        self.cmd.exec(Lease::from_dto(self.lease_dto, lpp))
    }

    fn unknown_lpn(self, symbol: SymbolOwned) -> Result<Self::Output, Self::Error> {
        self.cmd.unknown_lpn(symbol)
    }
}
