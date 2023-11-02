use currency::Currency;

use crate::{
    api::PositionSpecDTO,
    error::{ContractError, ContractResult},
    position::Spec,
};

impl<Lpn> From<Spec<Lpn>> for PositionSpecDTO
where
    Lpn: Currency,
{
    fn from(spec: Spec<Lpn>) -> Self {
        PositionSpecDTO::new_internal(
            spec.liability,
            spec.min_asset.into(),
            spec.min_trasaction_amount.into(),
        )
    }
}

impl<Lpn> TryFrom<PositionSpecDTO> for Spec<Lpn>
where
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionSpecDTO) -> ContractResult<Self> {
        Ok(Self::new(
            dto.liability,
            dto.min_asset.try_into()?,
            dto.min_sell_asset.try_into()?,
        ))
    }
}
