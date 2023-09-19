use currency::Currency;
use serde::{Deserialize, Serialize};

use crate::{
    api::{LeaseCoin, PositionSpec},
    error::{ContractError, ContractResult},
};

use super::Position;

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct PositionDTO {
    pub amount: LeaseCoin,
    spec: PositionSpec,
}

pub fn try_from<Asset, Lpn>(
    amount: &LeaseCoin,
    spec: PositionSpec,
) -> ContractResult<Position<Asset, Lpn>>
where
    Asset: Currency,
    Lpn: Currency,
{
    Ok(Position::new_internal(
        amount.try_into()?,
        spec.liability,
        spec.min_asset.try_into()?,
        spec.min_sell_asset.try_into()?,
    ))
}

impl<Asset, Lpn> TryFrom<PositionDTO> for Position<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionDTO) -> ContractResult<Self> {
        self::try_from(&dto.amount, dto.spec)
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
            spec: PositionSpec {
                liability: value.liability,
                min_asset: value.min_asset.into(),
                min_sell_asset: value.min_sell_asset.into(),
            },
        }
    }
}
