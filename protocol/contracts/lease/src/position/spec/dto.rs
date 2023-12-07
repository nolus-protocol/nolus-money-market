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
            spec.min_transaction.into(),
        )
    }
}

impl<Lpn> TryFrom<PositionSpecDTO> for Spec<Lpn>
where
    Lpn: Currency,
{
    type Error = ContractError;

    fn try_from(dto: PositionSpecDTO) -> ContractResult<Self> {
        dto.min_asset
            .try_into()
            .and_then(|min_asset| {
                dto.min_transaction
                    .try_into()
                    .map(|min_transaction| Self::new(dto.liability, min_asset, min_transaction))
            })
            .map_err(Into::into)
    }
}
