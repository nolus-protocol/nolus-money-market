use std::marker::PhantomData;

use currencies::{LeaseGroup, Lpns};
use currency::{AnyVisitor, AnyVisitorResult, Currency, GroupVisit, Tickers};

use super::LeaseDTO;

pub trait WithLeaseTypes {
    type Output;
    type Error;

    fn exec<Asset, Lpn>(self, lease_dto: LeaseDTO) -> Result<Self::Output, Self::Error>
    where
        Asset: Currency,
        Lpn: Currency;
}

pub fn execute<Cmd>(lease_dto: LeaseDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseTypes,
    currency::error::Error: Into<Cmd::Error>,
{
    Tickers.visit_any::<LeaseGroup, _>(
        &lease_dto.position.amount().ticker().clone(),
        FactoryStage1 { lease_dto, cmd },
    )
}

struct FactoryStage1<Cmd> {
    lease_dto: LeaseDTO,
    cmd: Cmd,
}

impl<Cmd> AnyVisitor for FactoryStage1<Cmd>
where
    Cmd: WithLeaseTypes,
    currency::error::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Asset>(self) -> AnyVisitorResult<Self>
    where
        Asset: Currency,
    {
        let lpn = self.lease_dto.loan.lpp().currency().to_owned();
        Tickers.visit_any::<Lpns, _>(
            &lpn,
            FactoryStage2 {
                lease_dto: self.lease_dto,
                cmd: self.cmd,
                asset: PhantomData::<Asset>,
            },
        )
    }
}
struct FactoryStage2<Cmd, Asset> {
    lease_dto: LeaseDTO,
    cmd: Cmd,
    asset: PhantomData<Asset>,
}

impl<Cmd, Asset> AnyVisitor for FactoryStage2<Cmd, Asset>
where
    Cmd: WithLeaseTypes,
    Asset: Currency,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Lpn>(self) -> AnyVisitorResult<Self>
    where
        Lpn: Currency,
    {
        self.cmd.exec::<Asset, Lpn>(self.lease_dto)
    }
}
