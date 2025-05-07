use currency::{CurrencyDef, MemberOf};

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::{LpnCurrencies, LpnCurrency},
    position::{Position, PositionError, WithPosition, WithPositionResult},
};

use super::LeaseDTO;

pub trait WithLeaseTypes {
    type Output;
    type Error;

    fn exec<Asset, Lpn>(
        self,
        lease_dto: LeaseDTO,
        position: Position<Asset>,
    ) -> Result<Self::Output, Self::Error>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies>,
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<LpnCurrencies>;
}

// TODO get rid of this function since the type params are known
pub fn execute<Cmd>(lease_dto: LeaseDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseTypes,
    PositionError: Into<Cmd::Error>,
{
    lease_dto
        .position
        .clone()
        .with_position(FactoryStage1 { lease_dto, cmd })
}

struct FactoryStage1<Cmd> {
    lease_dto: LeaseDTO,
    cmd: Cmd,
}

impl<Cmd> WithPosition for FactoryStage1<Cmd>
where
    Cmd: WithLeaseTypes,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Asset>(self, position: Position<Asset>) -> WithPositionResult<Self>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    {
        self.cmd
            .exec::<Asset, LpnCurrency>(self.lease_dto, position)
    }
}
