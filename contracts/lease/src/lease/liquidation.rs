use cosmwasm_std::Addr;

use finance::{
    coin::Coin,
    currency::{
        Currency,
        SymbolOwned
    },
    percent::Percent,
};

pub(crate) enum LiquidationStatus<Lpn>
where
    Lpn: Currency,
{
    None,
    FirstWarning(CommonInfo),
    SecondWarning(CommonInfo),
    ThirdWarning(CommonInfo),
    PartialLiquidation {
        _info: CommonInfo,
        _healthy_ltv: Percent,
        _liquidation_amount: Coin<Lpn>,
    },
    FullLiquidation(CommonInfo),
}

pub(crate) struct CommonInfo {
    pub customer: Addr,
    pub ltv: Percent,
    pub lease_asset: SymbolOwned,
}
