use cosmwasm_std::Addr;

use finance::{
    coin::Coin,
    currency::{
        Currency,
        SymbolOwned
    },
    percent::Percent,
};

pub enum LiquidationStatus<Lpn>
where
    Lpn: Currency,
{
    None,
    FirstWarning(WarningAndPartialLiquidationInfo),
    SecondWarning(WarningAndPartialLiquidationInfo),
    ThirdWarning(WarningAndPartialLiquidationInfo),
    PartialLiquidation(WarningAndPartialLiquidationInfo, Coin<Lpn>),
    FullLiquidation(WarningAndPartialLiquidationInfo, Coin<Lpn>),
}

pub struct WarningAndPartialLiquidationInfo {
    pub customer: Addr,
    pub ltv: Percent,
    pub ltv_healthy: Percent,
    pub lease_asset: SymbolOwned,
}
