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
    amount: LeaseCoin,
    spec: PositionSpec,
}

impl PositionDTO {
    pub fn amount(&self) -> &LeaseCoin {
        &self.amount
    }
}

impl<Asset, Lpn> TryFrom<PositionDTO> for Position<Asset, Lpn>
where
    Asset: Currency,
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionDTO) -> ContractResult<Self> {
        Self::try_from(dto.amount, dto.spec)
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
            spec: PositionSpec::new_internal(
                value.liability,
                value.min_asset.into(),
                value.min_sell_asset.into(),
            ),
        }
    }
}
