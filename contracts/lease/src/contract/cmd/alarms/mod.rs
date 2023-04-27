use finance::currency::Currency;
use platform::batch::{Emit, Emitter};

use crate::{
    event::Type,
    lease::{Cause, LeaseInfo, Status, WarningLevel},
};

pub mod price;
pub mod time;

fn emit_events<Lpn, Asset, L>(liquidation: &Status<Lpn>, lease: &L) -> Option<Emitter>
where
    Lpn: Currency,
    L: LeaseInfo,
{
    match liquidation {
        Status::None => None,
        Status::Warning(level) => Some(emit_warning(lease, level)),
        Status::PartialLiquidation { amount, cause } => {
            Some(emit_liquidation_start(lease, cause).emit_coin("amount", *amount))
        }
        Status::FullLiquidation(cause) => Some(emit_liquidation_start(lease, cause)),
    }
}

fn emit_lease<L>(emitter: Emitter, lease: &L) -> Emitter
where
    L: LeaseInfo,
{
    emitter
        .emit("customer", lease.customer())
        .emit("lease", lease.lease())
        .emit_currency::<_, L::Asset>("lease-asset")
}

fn emit_warning<Asset, L>(lease: &L, level: &WarningLevel) -> Emitter
where
    Asset: Currency,
    L: LeaseInfo<Asset = Asset>,
{
    emit_lease(Emitter::of_type(Type::LiquidationWarning), lease)
        .emit_percent_amount("ltv", level.ltv())
        .emit_to_string_value("level", level.ordinal())
}

fn emit_liquidation_start<L>(lease: &L, cause: &Cause) -> Emitter
where
    L: LeaseInfo,
{
    let emitter = emit_lease(Emitter::of_type(Type::LiquidationStart), lease);
    match cause {
        Cause::Liability { ltv, healthy_ltv } => emitter
            .emit("cause", "high liability")
            .emit_percent_amount("ltv", *ltv)
            .emit_percent_amount("ltv-healthy", *healthy_ltv),
        Cause::Overdue() => emitter.emit("cause", "overdue interest"),
    }
}

// fn emit_liquidation_info<Lpn>(emitter: Emitter, info: &LiquidationInfo<Lpn>) -> Emitter
// where
//     Lpn: Currency,
// {
//     emitter
//         .emit("of", info.lease.as_str())
//         .emit_coin("liquidation", info.receipt.total())
//         .emit_to_string_value("type", info.cause.to_uint())
//         .emit_coin_amount("prev-margin-interest", info.receipt.previous_margin_paid())
//         .emit_coin_amount("prev-loan-interest", info.receipt.previous_interest_paid())
//         .emit_coin_amount("curr-margin-interest", info.receipt.current_margin_paid())
//         .emit_coin_amount("curr-loan-interest", info.receipt.current_interest_paid())
//         .emit_coin_amount("principal", info.receipt.principal_paid())
//         .emit_coin_amount("excess", info.receipt.change())
// }

// fn emit_liquidation<Lpn, Asset, L>(
//     env: &Env,
//     lease: &L,
//     ltv: Percent,
//     liquidation_info: &LiquidationInfo<Lpn>,
// ) -> Emitter
// where
//     Lpn: Currency,
//     Asset: Currency,
//     L: LeaseInfo<Asset = Asset>,
// {
//     emit_liquidation_info(
//         emit_lease_info(Emitter::of_type(Type::Liquidation).emit_tx_info(env), lease)
//             .emit_percent_amount("ltv", ltv),
//         liquidation_info,
//     )
// }
