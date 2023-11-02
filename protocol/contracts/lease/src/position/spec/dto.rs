use currency::Currency;

use crate::{api::PositionSpecDTO, position::Spec};

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
