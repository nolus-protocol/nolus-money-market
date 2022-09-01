use serde::Serialize;

use finance::currency::Currency;
use platform::{
    batch::{Batch, Emit, Emitter},
    utils::Either,
};

use crate::{
    event::TYPE,
    lease::{LeaseDTO, LeaseInfo, Status, WarningLevel},
};

pub mod price;

pub struct LiquidationResult {
    pub(super) response: Either<Batch, Emitter>,
    pub(super) lease_dto: LeaseDTO,
}

fn emit_events<Lpn>(liquidation: &Status<Lpn>, batch: Batch) -> Either<Batch, Emitter>
where
    Lpn: Currency + Serialize,
{
    Either::Right(match liquidation {
        Status::None => return Either::Left(batch),
        &Status::Warning(ref info, level) => emit_warning(batch, info, level),
        Status::PartialLiquidation { .. } => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation)
        }
        Status::FullLiquidation(_) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation)
        }
    })
}

fn emit_warning(batch: Batch, info: &LeaseInfo, level: WarningLevel) -> Emitter {
    batch
        .into_emitter(TYPE::LiquidationWarning)
        .emit("customer", &info.customer)
        .emit_percent_amount("ltv", info.ltv)
        .emit_to_string_value("level", level.to_uint())
        .emit("lease-asset", &info.lease_asset)
}
