use finance::{currency::Currency, percent::Percent};
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::Env;

use crate::{
    event::Type,
    lease::{LeaseInfo, LiquidationInfo, Status, WarningLevel},
};

pub mod price;
pub mod time;

fn emit_events<Lpn, Asset, L>(env: &Env, liquidation: &Status<Lpn>, lease: &L) -> Option<Emitter>
where
    Lpn: Currency,
    L: LeaseInfo,
{
    match liquidation {
        Status::None => None,
        &Status::Warning { ltv, level } => Some(emit_warning(lease, ltv, level)),
        Status::PartialLiquidation {
            ltv,
            liquidation_info,
            healthy_ltv,
        } => Some(
            emit_liquidation(env, lease, *ltv, liquidation_info)
                .emit_percent_amount("ltv-healthy", *healthy_ltv),
        ),
        Status::FullLiquidation {
            ltv,
            liquidation_info,
        } => Some(emit_liquidation(env, lease, *ltv, liquidation_info)),
    }
}

fn emit_lease_info<Asset, L>(emitter: Emitter, lease: &L, ltv: Percent) -> Emitter
where
    Asset: Currency,
    L: LeaseInfo<Asset = Asset>,
{
    emitter
        .emit("customer", lease.customer())
        .emit("lease", lease.lease())
        .emit_percent_amount("ltv", ltv)
        .emit_currency::<_, Asset>("lease-asset")
}

fn emit_warning<Asset, L>(lease: &L, ltv: Percent, level: WarningLevel) -> Emitter
where
    Asset: Currency,
    L: LeaseInfo<Asset = Asset>,
{
    emit_lease_info(Emitter::of_type(Type::LiquidationWarning), lease, ltv)
        .emit_to_string_value("level", level.to_uint())
}

fn emit_liquidation_info<Lpn>(emitter: Emitter, info: &LiquidationInfo<Lpn>) -> Emitter
where
    Lpn: Currency,
{
    emitter
        .emit("of", info.lease.as_str())
        .emit_coin("liquidation", info.receipt.total())
        .emit_to_string_value("type", info.cause.to_uint())
        .emit_coin_amount("prev-margin-interest", info.receipt.previous_margin_paid())
        .emit_coin_amount("prev-loan-interest", info.receipt.previous_interest_paid())
        .emit_coin_amount("curr-margin-interest", info.receipt.current_margin_paid())
        .emit_coin_amount("curr-loan-interest", info.receipt.current_interest_paid())
        .emit_coin_amount("principal", info.receipt.principal_paid())
        .emit_coin_amount("excess", info.receipt.change())
}

fn emit_liquidation<Lpn, Asset, L>(
    env: &Env,
    lease: &L,
    ltv: Percent,
    liquidation_info: &LiquidationInfo<Lpn>,
) -> Emitter
where
    Lpn: Currency,
    Asset: Currency,
    L: LeaseInfo<Asset = Asset>,
{
    emit_liquidation_info(
        emit_lease_info(
            Emitter::of_type(Type::Liquidation).emit_tx_info(env),
            lease,
            ltv,
        ),
        liquidation_info,
    )
}
