use serde::{Deserialize, Serialize};

use currency::{CurrencyDef, MemberOf};
use finance::liability::Zone;
use platform::batch::Batch;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarmsRef;

use crate::{
    api::{LeaseAssetCurrencies, LeaseCoin},
    error::ContractResult,
    finance::OracleRef,
    lease::CloseStatus,
    position::{Cause, CloseStrategy, Liquidation},
};

pub(crate) mod change;
pub(crate) mod check;

pub(crate) enum CloseStatusDTO {
    Paid,
    None {
        current_liability: Zone,
        alarms: Batch,
    },
    NeedLiquidation(LiquidationDTO),
    CloseAsked(CloseStrategy),
}

#[derive(Serialize, Deserialize)]
pub(crate) enum LiquidationDTO {
    Partial(PartialLiquidationDTO),
    Full(FullLiquidationDTO),
}

#[derive(Serialize, Deserialize)]
pub(crate) struct PartialLiquidationDTO {
    pub amount: LeaseCoin,
    pub cause: Cause,
}
#[derive(Serialize, Deserialize)]
pub(crate) struct FullLiquidationDTO {
    pub cause: Cause,
}

impl CloseStatusDTO {
    fn try_from_do<Asset>(
        status: CloseStatus<Asset>,
        when: &Timestamp,
        time_alarms: &TimeAlarmsRef,
        price_alarms: &OracleRef,
    ) -> ContractResult<Self>
    where
        Asset: CurrencyDef,
        Asset::Group: MemberOf<LeaseAssetCurrencies>,
    {
        match status {
            CloseStatus::Paid => Ok(Self::Paid),
            CloseStatus::None {
                current_liability,
                steadiness,
            } => steadiness
                .try_into_alarms(when, time_alarms, price_alarms)
                .map(|alarms| Self::None {
                    current_liability,
                    alarms,
                }),
            CloseStatus::NeedLiquidation(liquidation) => {
                Ok(Self::NeedLiquidation(liquidation.into()))
            }
            CloseStatus::CloseAsked(strategy) => Ok(Self::CloseAsked(strategy)),
        }
    }
}

impl<Asset> From<Liquidation<Asset>> for LiquidationDTO
where
    Asset: CurrencyDef,
    Asset::Group: MemberOf<LeaseAssetCurrencies>,
{
    fn from(value: Liquidation<Asset>) -> Self {
        match value {
            Liquidation::Partial { amount, cause } => Self::Partial(PartialLiquidationDTO {
                amount: amount.into(),
                cause,
            }),
            Liquidation::Full(cause) => Self::Full(FullLiquidationDTO { cause }),
        }
    }
}
