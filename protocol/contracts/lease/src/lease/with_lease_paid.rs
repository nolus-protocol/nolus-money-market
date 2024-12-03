use currency::{AnyVisitor, AnyVisitorResult, CurrencyDTO, CurrencyDef, MemberOf};

use crate::{
    api::{LeaseAssetCurrencies, LeasePaymentCurrencies},
    finance::LpnCurrencies,
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

pub fn execute<Cmd>(lease_dto: LeaseDTO, cmd: Cmd) -> Result<Cmd::Output, Cmd::Error>
where
    Cmd: WithLeaseTypes,
    finance::error::Error: Into<Cmd::Error>,
    currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
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
    // Cmd::Error: From<lpp::error::ContractError>,
    currency::error::Error: Into<Cmd::Error>,
    oracle_platform::error::Error: Into<Cmd::Error>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Asset>(self, position: Position<Asset>) -> WithPositionResult<Self>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies> + MemberOf<LeasePaymentCurrencies>,
    {
        let lpn = self.lease_dto.loan.lpp().lpn().to_owned();
        lpn.into_currency_type(FactoryStage2 {
            lease_dto: self.lease_dto,
            cmd: self.cmd,
            position,
        })
    }
}
struct FactoryStage2<Cmd, Asset> {
    lease_dto: LeaseDTO,
    cmd: Cmd,
    position: Position<Asset>,
}

impl<Cmd, Asset> AnyVisitor<LpnCurrencies> for FactoryStage2<Cmd, Asset>
where
    Cmd: WithLeaseTypes,
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies>,
{
    type Output = Cmd::Output;
    type Error = Cmd::Error;

    fn on<Lpn>(self, _def: &CurrencyDTO<Lpn::Group>) -> AnyVisitorResult<LpnCurrencies, Self>
    where
        Lpn: CurrencyDef,
        Lpn::Group: MemberOf<LpnCurrencies>,
    {
        self.cmd.exec::<Asset, Lpn>(self.lease_dto, self.position)
    }
}
