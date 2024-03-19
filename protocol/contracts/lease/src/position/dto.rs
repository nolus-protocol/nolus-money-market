use currency::Currency;
use serde::{Deserialize, Serialize};

use crate::{
    api::{open::PositionSpecDTO, LeaseCoin},
    error::{ContractError, ContractResult},
};

use super::{Position, Spec};

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(test, derive(Debug))]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PositionDTO {
    amount: LeaseCoin,
    spec: PositionSpecDTO,
}

impl PositionDTO {
    pub fn amount(&self) -> &LeaseCoin {
        &self.amount
    }
}

impl<Asset> TryFrom<PositionDTO> for Position<Asset>
where
    Asset: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionDTO) -> ContractResult<Self> {
        Spec::try_from(dto.spec).and_then(|spec| Self::try_from(dto.amount, spec))
    }
}

impl<Asset> From<Position<Asset>> for PositionDTO
where
    Asset: Currency,
{
    fn from(value: Position<Asset>) -> Self {
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
