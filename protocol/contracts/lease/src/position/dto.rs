use currency::Currency;
use serde::{Deserialize, Serialize};

use crate::{
    api::{LeaseCoin, PositionSpecDTO},
    error::{ContractError, ContractResult},
};

use super::{Position, Spec};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
pub struct PositionDTO {
    amount: LeaseCoin,
    spec: PositionSpecDTO,
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
        Self::try_from(dto.amount, Spec::try_from(dto.spec)?)
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
            spec: value.spec.into(),
        }
    }
}

impl From<PositionDTO> for LeaseCoin {
    fn from(value: PositionDTO) -> Self {
        value.amount().clone()
    }
}
