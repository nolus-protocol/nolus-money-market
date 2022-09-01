use serde::Serialize;

use finance::currency::Currency;
use platform::{
    batch::{Batch, Emit, Emitter},
    utils::Either
};

use crate::{
    event::TYPE,
    lease::{CommonInfo, LeaseDTO, LiquidationStatus, WarningLevel},
};

pub mod price_alarm;

pub struct LiquidationResult
{
    pub(super) response: Either<Batch, Emitter>,
    pub(super) lease_dto: LeaseDTO,
}

fn emit_events<Lpn>(liquidation: &LiquidationStatus<Lpn>, batch: Batch) -> Either<Batch, Emitter>
where
    Lpn: Currency + Serialize,
{
    Either::Right(match liquidation {
        LiquidationStatus::None => return Either::Left(batch.into()),
        &LiquidationStatus::Warning(ref info, level) => emit_warning(batch, info, level),
        LiquidationStatus::PartialLiquidation { .. } => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
        LiquidationStatus::FullLiquidation(_) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation).into()
        }
    })
}

fn emit_warning(batch: Batch, info: &CommonInfo, level: WarningLevel) -> Emitter {
    batch
        .into_emitter(TYPE::LiquidationWarning)
        .emit("customer", &info.customer)
        .emit_percent_amount("ltv", info.ltv)
        .emit_to_string_value("level", level as u8)
        .emit("lease-asset", &info.lease_asset)
        .into()
}
