use finance::{currency::Currency, liability::Level};
use lpp::stub::lender::LppLender as LppLenderTrait;
use oracle::stub::Oracle as OracleTrait;
use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
};
use profit::stub::Profit as ProfitTrait;
use sdk::cosmwasm_std::Timestamp;
use timealarms::stub::TimeAlarms as TimeAlarmsTrait;

use serde::Serialize;

use crate::{
    error::ContractError,
    event::Type,
    lease::{with_lease::WithLease, Cause, IntoDTOResult, Lease, LeaseDTO, Status},
};

pub struct LiquidationStatus {
    now: Timestamp,
}

impl LiquidationStatus {
    pub fn new(now: Timestamp) -> Self {
        Self { now }
    }
}

impl WithLease for LiquidationStatus {
    type Output = MessageResponse;

    type Error = ContractError;

    fn exec<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>(
        self,
        mut lease: Lease<Lpn, Asset, Lpp, Profit, TimeAlarms, Oracle>,
    ) -> Result<Self::Output, Self::Error>
    where
        Lpn: Currency + Serialize,
        Lpp: LppLenderTrait<Lpn>,
        TimeAlarms: TimeAlarmsTrait,
        Oracle: OracleTrait<Lpn>,
        Profit: ProfitTrait,
        Asset: Currency + Serialize,
    {
        let status = lease.liquidation_status(self.now)?;
        match status {
            Status::No(zone) => lease.reschedule(&self.now, &zone)?,
            Status::Partial {
                amount: _,
                cause: _,
            } => todo!("init liquidation"),
            Status::Full(_cause) => todo!("init liquidation"),
        }

        let IntoDTOResult { batch, lease } = lease.into_dto();
        Ok(
            if let Some(events) = emit_events::<_, Asset>(&status, lease) {
                MessageResponse::messages_with_events(batch, events)
            } else {
                MessageResponse::messages_only(batch)
            },
        )
    }
}

fn emit_events<Lpn, Asset>(liquidation: &Status<Lpn>, lease: LeaseDTO) -> Option<Emitter>
where
    Lpn: Currency,
    Asset: Currency,
{
    match liquidation {
        Status::No(zone) => zone
            .low()
            .map(|low_level| emit_warning::<Asset>(lease, &low_level)),
        Status::Partial { amount, cause } => {
            Some(emit_liquidation_start::<Asset>(lease, cause).emit_coin("amount", *amount))
        }
        Status::Full(cause) => Some(emit_liquidation_start::<Asset>(lease, cause)),
    }
}

fn emit_lease<Asset>(emitter: Emitter, lease: LeaseDTO) -> Emitter
where
    Asset: Currency,
{
    emitter
        .emit("customer", lease.customer)
        .emit("lease", lease.addr)
        .emit_currency::<_, Asset>("lease-asset")
}

fn emit_warning<Asset>(lease: LeaseDTO, level: &Level) -> Emitter
where
    Asset: Currency,
{
    emit_lease::<Asset>(Emitter::of_type(Type::LiquidationWarning), lease)
        .emit_percent_amount("ltv", level.ltv())
        .emit_to_string_value("level", level.ordinal())
}

fn emit_liquidation_start<Asset>(lease: LeaseDTO, cause: &Cause) -> Emitter
where
    Asset: Currency,
{
    let emitter = emit_lease::<Asset>(Emitter::of_type(Type::LiquidationStart), lease);
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
