use cosmwasm_std::Response;
use serde::Serialize;

use finance::currency::Currency;
use platform::batch::{Batch, Emit};

use crate::{
    event::TYPE,
    lease::{LeaseDTO, LiquidationStatus},
};

pub mod price_alarm;

fn emit_events<Lpn>(liquidation: &LiquidationStatus<Lpn>, batch: Batch) -> Response where Lpn: Currency + Serialize {
    match &liquidation {
        LiquidationStatus::None => batch.into(),
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
                    },
                )
                .emit("lease-asset", &info.lease_asset)
                .into()
        }
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

pub struct LiquidationResult
{
    pub(super) response: Response,
    pub(super) lease: LeaseDTO,
}
