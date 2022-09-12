use cosmwasm_std::Response;

use finance::currency::Currency;
use platform::batch::{Batch, Emit};

use crate::{
    event::TYPE,
    lease::{LeaseDTO, LeaseInfo, Status, WarningLevel},
};

pub mod price;
pub mod time;

pub struct AlarmResult {
    pub(super) response: Response,
    pub(super) lease_dto: LeaseDTO,
}

fn emit_events<Lpn>(liquidation: &Status<Lpn>, batch: Batch) -> Response
where
    Lpn: Currency,
{
    match liquidation {
        Status::None => batch.into(),
        &Status::Warning(ref info, level) => emit_warning(batch, info, level),
        Status::PartialLiquidation { .. } => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
        Status::FullLiquidation(_) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
    }
}

fn emit_warning<Asset>(batch: Batch, info: &LeaseInfo<Asset>, level: WarningLevel) -> Response
where
    Asset: Currency,
{
    batch
        .into_emitter(TYPE::LiquidationWarning)
        .emit("customer", &info.customer)
        .emit_percent_amount("ltv", info.ltv)
        .emit_to_string_value("level", level.to_uint())
        .emit("lease-asset", Asset::SYMBOL)
        .into()
}
