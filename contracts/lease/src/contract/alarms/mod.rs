use cosmwasm_std::Response;
use serde::Serialize;

use finance::currency::Currency;
use platform::batch::{Batch, Emit};

use crate::{
    event::TYPE,
    lease::{CommonInfo, LeaseDTO, LiquidationStatus, WarningLevel},
};

pub mod price_alarm;

pub struct LiquidationResult
{
    pub(super) response: Response,
    pub(super) lease_dto: LeaseDTO,
}

fn emit_events<Lpn>(liquidation: &LiquidationStatus<Lpn>, batch: Batch) -> Response
where
    Lpn: Currency + Serialize,
{
    match liquidation {
        LiquidationStatus::None => batch.into(),
        &LiquidationStatus::Warning(ref info, level) => emit_warning(batch, info, level),
        LiquidationStatus::PartialLiquidation { .. } => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
        LiquidationStatus::FullLiquidation(_) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
    }
}

fn emit_warning(batch: Batch, info: &CommonInfo, level: WarningLevel) -> Response {
    batch
        .into_emitter(TYPE::LiquidationWarning)
        .emit("customer", &info.customer)
        .emit_percent_amount("ltv", info.ltv)
        .emit_to_string_value("level", level as u8)
        .emit("lease-asset", &info.lease_asset)
        .into()
}
