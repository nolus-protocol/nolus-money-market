use currency::Currency;
use serde::{Deserialize, Serialize};

use finance::liability::Liability;

use crate::{
    api::{LeaseCoin, LpnCoin},
    error::{ContractError, ContractResult},
};

use super::Position;

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct PositionDTO {
    pub amount: LeaseCoin,
    pub liability: Liability,
    min_asset: LpnCoin,
    min_sell_asset: LpnCoin,
}

impl<Asset, Lpn> TryFrom<PositionDTO> for Position<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionDTO) -> ContractResult<Self> {
        Ok(Position::new(
            dto.amount.try_into()?,
            dto.liability,
            dto.min_asset.try_into()?,
            dto.min_sell_asset.try_into()?,
        ))
    }
}

impl<Asset, Lpn> From<Position<Asset, Lpn>> for PositionDTO
where
    Asset: Currency,
    Lpn: Currency,
{
    fn from(value: Position<Asset, Lpn>) -> Self {
        Self {
            amount: value.amount.into(),
            liability: value.liability,
            min_asset: value.min_asset.into(),
            min_sell_asset: value.min_sell_asset.into(),
        }
    }
}
