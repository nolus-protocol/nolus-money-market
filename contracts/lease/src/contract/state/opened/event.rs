use finance::liability::Level;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::Env;

use crate::{
    api::DownpaymentCoin,
    contract::cmd::{LiquidationDTO, OpenLoanRespResult, ReceiptDTO},
    event::Type,
    lease::{Cause, LeaseDTO},
};

pub(super) fn emit_lease_opened(
    env: &Env,
    lease: &LeaseDTO,
    loan: OpenLoanRespResult,
    downpayment: DownpaymentCoin,
) -> Emitter {
    Emitter::of_type(Type::OpenedActive)
        .emit_tx_info(env)
        .emit("id", &lease.addr)
        .emit("customer", lease.customer.clone())
        .emit_percent_amount(
            "air",
            loan.annual_interest_rate + lease.loan.annual_margin_interest(),
        )
        .emit("currency", lease.amount.ticker())
        .emit("loan-pool-id", lease.loan.lpp().addr())
        .emit_coin_dto("loan", &loan.principal)
        .emit_coin_dto("downpayment", &downpayment)
}

pub(super) fn emit_payment(env: &Env, lease: &LeaseDTO, receipt: &ReceiptDTO) -> Emitter {
    Emitter::of_type(Type::PaidActive)
        .emit_tx_info(env)
        .emit("to", lease.addr.clone())
        .emit_coin_dto("payment", &receipt.total)
        .emit_to_string_value("loan-close", receipt.close)
        .emit_coin_amount(
            "prev-margin-interest",
            receipt.previous_margin_paid.amount(),
        )
        .emit_coin_amount(
            "prev-loan-interest",
            receipt.previous_interest_paid.amount(),
        )
        .emit_coin_amount("curr-margin-interest", receipt.current_margin_paid.amount())
        .emit_coin_amount("curr-loan-interest", receipt.current_interest_paid.amount())
        .emit_coin_amount("principal", receipt.principal_paid.amount())
        .emit_coin_amount("change", receipt.change.amount())
}

pub(super) fn emit_liquidation_warning(lease: &LeaseDTO, level: &Level) -> Emitter {
    emit_lease(Emitter::of_type(Type::LiquidationWarning), lease)
        .emit_percent_amount("ltv", level.ltv())
        .emit_to_string_value("level", level.ordinal())
}

pub(super) fn emit_liquidation_start(lease: &LeaseDTO, liquidation: &LiquidationDTO) -> Emitter {
    let emitter = emit_lease(Emitter::of_type(Type::LiquidationStart), lease);
    match liquidation {
        LiquidationDTO::Partial { amount, cause } => {
            emit_liquidation_cause(emitter, cause).emit_coin_dto("amount", amount)
        }
        LiquidationDTO::Full(cause) => emit_liquidation_cause(emitter, cause),
    }
}

fn emit_liquidation_cause(emitter: Emitter, cause: &Cause) -> Emitter {
    match cause {
        Cause::Liability { ltv, healthy_ltv } => emitter
            .emit("cause", "high liability")
            .emit_percent_amount("ltv", *ltv)
            .emit_percent_amount("ltv-healthy", *healthy_ltv),
        Cause::Overdue() => emitter.emit("cause", "overdue interest"),
    }
}

fn emit_lease(emitter: Emitter, lease: &LeaseDTO) -> Emitter {
    emitter
        .emit("customer", lease.customer.clone())
        .emit("lease", lease.addr.clone())
        .emit_currency_symbol("lease-asset", lease.amount.ticker())
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
