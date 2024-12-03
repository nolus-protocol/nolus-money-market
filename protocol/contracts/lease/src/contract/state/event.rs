use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::{
    api::LeaseCoin, contract::cmd::RepayEmitter, event::Type, loan::RepayReceipt, position::Cause,
};

pub(crate) struct LiquidationEmitter<'liq, 'env> {
    cause: &'liq Cause,
    amount: LeaseCoin,
    env: &'env Env,
}

impl<'liq, 'env> LiquidationEmitter<'liq, 'env> {
    pub fn new(cause: &'liq Cause, amount: LeaseCoin, env: &'env Env) -> Self {
        Self { cause, amount, env }
    }
}
impl RepayEmitter for LiquidationEmitter<'_, '_> {
    fn emit(self, lease: &Addr, receipt: &RepayReceipt) -> Emitter {
        let emitter = emit_payment_int(Type::Liquidation, self.env, lease, receipt);
        emit_liquidation_cause(emitter, self.cause).emit_coin_dto("amount", &self.amount)
    }
}

pub(crate) struct PositionCloseEmitter<'env> {
    amount: LeaseCoin,
    env: &'env Env,
}

impl<'env> PositionCloseEmitter<'env> {
    pub fn new(amount: LeaseCoin, env: &'env Env) -> Self {
        Self { amount, env }
    }
}
impl RepayEmitter for PositionCloseEmitter<'_> {
    fn emit(self, lease: &Addr, receipt: &RepayReceipt) -> Emitter {
        let emitter = emit_payment_int(Type::ClosePosition, self.env, lease, receipt);
        emitter.emit_coin_dto("amount", &self.amount)
    }
}

pub(super) fn emit_payment_int(
    event_type: Type,
    env: &Env,
    lease: &Addr,
    receipt: &RepayReceipt,
) -> Emitter {
    Emitter::of_type(event_type)
        .emit_tx_info(env)
        .emit("to", lease)
        .emit_coin("payment", receipt.total())
        .emit_to_string_value("loan-close", receipt.close())
        .emit_coin_amount("overdue-margin-interest", receipt.overdue_margin_paid())
        .emit_coin_amount("overdue-loan-interest", receipt.overdue_interest_paid())
        .emit_coin_amount("due-margin-interest", receipt.due_margin_paid())
        .emit_coin_amount("due-loan-interest", receipt.due_interest_paid())
        .emit_coin_amount("principal", receipt.principal_paid())
        .emit_coin_amount("change", receipt.change())
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
