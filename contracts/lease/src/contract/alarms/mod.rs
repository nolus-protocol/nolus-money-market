use cosmwasm_std::{Env, Response};

use finance::currency::Currency;
use platform::batch::{Batch, Emit, Emitter};

use crate::{
    event::TYPE,
    lease::{LeaseDTO, LeaseInfo, LiquidationCause, LiquidationInfo, Status, WarningLevel},
};

pub mod price;
pub mod time;

pub struct AlarmResult {
    pub(super) response: Response,
    pub(super) lease_dto: LeaseDTO,
}

fn emit_events<Lpn, Asset>(env: &Env, liquidation: &Status<Lpn, Asset>, batch: Batch) -> Response
where
    Lpn: Currency,
    Asset: Currency,
{
    match liquidation {
        Status::None => batch.into(),
        &Status::Warning(ref info, level) => emit_warning(batch, info, level),
        Status::PartialLiquidation {
            info,
            liquidation_info,
            healthy_ltv,
        } => emit_liquidation(batch, env, info, liquidation_info)
            .emit_percent_amount("ltv-healthy", *healthy_ltv)
            .into(),
        Status::FullLiquidation {
            info,
            liquidation_info,
        } => emit_liquidation(batch, env, info, liquidation_info).into(),
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
        .emit_currency::<_, Asset>("lease-asset")
        .into()
}

fn emit_lease_info<Asset>(emitter: Emitter, info: &LeaseInfo<Asset>) -> Emitter
where
    Asset: Currency,
{
    emitter
        .emit("customer", &info.customer)
        .emit_percent_amount("ltv", info.ltv)
        .emit_currency::<_, Asset>("lease-asset")
}

fn emit_liquidation_info<Lpn>(mut emitter: Emitter, info: &LiquidationInfo<Lpn>) -> Emitter
where
    Lpn: Currency,
{
    emitter = emitter
        .emit("of", info.lease.as_str())
        .emit_coin("liquidation", info.receipt.total())
        .emit_to_string_value("type", info.cause.to_uint())
        .emit_coin_amount("prev-margin-interest", info.receipt.previous_margin_paid())
        .emit_coin_amount("prev-loan-interest", info.receipt.previous_interest_paid());

    if matches!(info.cause, LiquidationCause::Liability) {
        emitter
            .emit_coin_amount("curr-margin-interest", info.receipt.current_margin_paid())
            .emit_coin_amount("curr-loan-interest", info.receipt.current_interest_paid())
            .emit_coin_amount("principal", info.receipt.principal_paid())
    } else {
        emitter
    }
}

fn emit_liquidation<Lpn, Asset>(
    batch: Batch,
    env: &Env,
    lease_info: &LeaseInfo<Asset>,
    liquidation_info: &LiquidationInfo<Lpn>,
) -> Emitter
where
    Lpn: Currency,
    Asset: Currency,
{
    emit_liquidation_info(
        emit_lease_info(
            batch.into_emitter(TYPE::Liquidation).emit_tx_info(env),
            lease_info,
        ),
        liquidation_info,
    )
}
