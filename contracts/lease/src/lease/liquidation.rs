use cosmwasm_std::Addr;

use finance::{
    coin::Coin,
    currency::{
        Currency,
        SymbolOwned
    },
    percent::Percent,
};
use platform::batch::Batch;

use super::LeaseDTO;

pub(crate) enum LiquidationStatus<Lpn>
where
    Lpn: Currency,
{
    None,
    Warning(CommonInfo, WarningLevel),
    PartialLiquidation {
        _info: CommonInfo,
        _healthy_ltv: Percent,
        _liquidation_amount: Coin<Lpn>,
    },
    FullLiquidation(CommonInfo),
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(u8)]
pub(crate) enum WarningLevel {
    First = 1,
    Second = 2,
    Third = 3,
}

pub(crate) struct CommonInfo {
    pub customer: Addr,
    pub ltv: Percent,
    pub lease_asset: SymbolOwned,
}

pub(crate) struct OnAlarmResult<Lpn>
where
    Lpn: Currency,
{
    pub batch: Batch,
    pub lease_dto: LeaseDTO,
    pub liquidation_status: LiquidationStatus<Lpn>,
}
