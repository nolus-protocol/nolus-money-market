use finance::currency::Currency;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::Env;

use crate::{
    event::Type,
    lease::{LeaseDTO, LeaseInfo, LiquidationInfo, Status, WarningLevel},
};

pub mod price;
pub mod time;

pub struct AlarmResult {
    pub response: MessageResponse,
    pub lease_dto: LeaseDTO,
}

fn emit_events<Lpn, Asset>(
    env: &Env,
    liquidation: &Status<Lpn, Asset>,
    batch: Batch,
) -> MessageResponse
where
    Lpn: Currency,
    Asset: Currency,
{
    match liquidation {
        Status::None => batch.into(),
        &Status::Warning(ref info, level) => {
            MessageResponse::messages_with_events(batch, emit_warning(info, level))
        }
        Status::PartialLiquidation {
            info,
            liquidation_info,
            healthy_ltv,
        } => MessageResponse::messages_with_events(
            batch,
            emit_liquidation(env, info, liquidation_info)
                .emit_percent_amount("ltv-healthy", *healthy_ltv),
        ),
        Status::FullLiquidation {
            info,
            liquidation_info,
        } => MessageResponse::messages_with_events(
            batch,
            emit_liquidation(env, info, liquidation_info),
        ),
    }
}

fn emit_lease_info<Asset>(emitter: Emitter, info: &LeaseInfo<Asset>) -> Emitter
where
    Asset: Currency,
{
    emitter
        .emit("customer", &info.customer)
        .emit("lease", &info.lease)
        .emit_percent_amount("ltv", info.ltv)
        .emit_currency::<_, Asset>("lease-asset")
}

fn emit_warning<Asset>(info: &LeaseInfo<Asset>, level: WarningLevel) -> Emitter
where
    Asset: Currency,
{
    emit_lease_info(Emitter::of_type(Type::LiquidationWarning), info)
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

fn emit_liquidation<Lpn, Asset>(
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
            Emitter::of_type(Type::Liquidation).emit_tx_info(env),
            lease_info,
        ),
        liquidation_info,
    )
}
