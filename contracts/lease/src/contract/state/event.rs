use finance::liability::Cause;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::{api::LeaseCoin, contract::cmd::ReceiptDTO, event::Type};

pub(super) fn emit_liquidation(
    env: &Env,
    lease_addr: &Addr,
    receipt: &ReceiptDTO,
    liquidation_cause: &Cause,
    liquidation_amount: &LeaseCoin,
) -> Emitter {
    let emitter = emit_payment_int(Type::Liquidation, env, lease_addr, receipt);
    emit_liquidation_info(emitter, liquidation_cause, liquidation_amount)
}

pub(super) fn emit_payment_int(
    event_type: Type,
    env: &Env,
    lease_addr: &Addr,
    receipt: &ReceiptDTO,
) -> Emitter {
    Emitter::of_type(event_type)
        .emit_tx_info(env)
        .emit("to", lease_addr)
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

pub(super) fn emit_liquidation_info(
    emitter: Emitter,
    liquidation_cause: &Cause,
    liquidation_amount: &LeaseCoin,
) -> Emitter {
    let emitter = emit_liquidation_cause(emitter, liquidation_cause);
    emitter.emit_coin_dto("amount", liquidation_amount)
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
