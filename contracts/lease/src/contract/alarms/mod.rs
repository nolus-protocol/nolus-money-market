use serde::Serialize;

use finance::currency::Currency;
use platform::{
    batch::{Batch, Emit, Emitter},
    either::Either
};

use crate::{
    event::TYPE,
    lease::{LeaseDTO, LiquidationStatus},
};

pub mod price_alarm;

fn emit_events<Lpn>(liquidation: &LiquidationStatus<Lpn>, batch: Batch) -> Either<Batch, Emitter> where Lpn: Currency + Serialize {
    Either::Right(match &liquidation {
        LiquidationStatus::None => return Either::Left(batch),
        warning @ LiquidationStatus::FirstWarning(info)
        | warning @ LiquidationStatus::SecondWarning(info)
        | warning @ LiquidationStatus::ThirdWarning(info) => {

                batch
                    .into_emitter(TYPE::LiquidationWarning)
                    .emit("customer", &info.customer)
                    .emit_percent_amount("ltv", info.ltv)
                    .emit_to_string_value(
                        "level",
                        match warning {
                            LiquidationStatus::FirstWarning(_) => 1,
                            LiquidationStatus::SecondWarning(_) => 2,
                            LiquidationStatus::ThirdWarning(_) => 3,
                            _ => unreachable!(),
                        }
                    )
                    .emit("lease-asset", &info.lease_asset)
        }
        LiquidationStatus::PartialLiquidation(..) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation)
        }
        LiquidationStatus::FullLiquidation(..) => {
            // TODO add event attributes
            batch.into_emitter(TYPE::Liquidation)
        }
    })
}

pub struct LiquidationResult
{
    pub(super) into_response: Either<Batch, Emitter>,
    pub(super) lease: LeaseDTO,
}
