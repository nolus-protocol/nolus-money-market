use serde::Serialize;

use finance::{
    currency::Currency,
    ratio::Ratio
};
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
    match &liquidation {
        LiquidationStatus::None => Either::Left(batch),
        warning @ LiquidationStatus::FirstWarning(ltv)
        | warning @ LiquidationStatus::SecondWarning(ltv)
        | warning @ LiquidationStatus::ThirdWarning(ltv) => {
            Either::Right(
                batch
                    .into_emitter(TYPE::LiquidationWarning)
                    .emit_to_string_value("current-ltv", ltv.parts())
                    .emit_to_string_value(
                        "warning-level",
                        match warning {
                            LiquidationStatus::FirstWarning(_) => 1,
                            LiquidationStatus::SecondWarning(_) => 2,
                            LiquidationStatus::ThirdWarning(_) => 3,
                            _ => unreachable!(),
                        }
                    )
            )
        }
        LiquidationStatus::PartialLiquidation(_) => {
            let emitter = batch.into_emitter(TYPE::Liquidation);

            // TODO add event attributes

            Either::Right(emitter)
        }
        LiquidationStatus::FullLiquidation(_) => {
            let emitter = batch.into_emitter(TYPE::Liquidation);

            // TODO add event attributes

            Either::Right(emitter)
        }
    }
}

pub struct LiquidationResult
{
    pub(super) into_response: Either<Batch, Emitter>,
    pub(super) lease: LeaseDTO,
}
