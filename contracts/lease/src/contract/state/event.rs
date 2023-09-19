use currency::Currency;
use finance::liability::Cause;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::{api::LeaseCoin, contract::cmd::RepayEmitter, event::Type, loan::RepayReceipt};

pub(crate) struct LiquidationEmitter<'liq, 'env> {
    cause: &'liq Cause,
    amount: &'liq LeaseCoin,
    env: &'env Env,
}

impl<'liq, 'env> LiquidationEmitter<'liq, 'env> {
    pub fn new(cause: &'liq Cause, amount: &'liq LeaseCoin, env: &'env Env) -> Self {
        Self { cause, amount, env }
    }
}
impl<'liq, 'env> RepayEmitter for LiquidationEmitter<'liq, 'env> {
    fn emit<Lpn>(self, lease: &Addr, receipt: &RepayReceipt<Lpn>) -> Emitter
    where
        Lpn: Currency,
    {
        let emitter = emit_payment_int(Type::Liquidation, self.env, lease, receipt);
        emit_liquidation_cause(emitter, self.cause).emit_coin_dto("amount", self.amount)
    }
}

pub(super) fn emit_payment_int<Lpn>(
    event_type: Type,
    env: &Env,
    lease: &Addr,
    receipt: &RepayReceipt<Lpn>,
) -> Emitter
where
    Lpn: Currency,
{
    Emitter::of_type(event_type)
        .emit_tx_info(env)
        .emit("to", lease)
        .emit_coin("payment", receipt.total())
        .emit_to_string_value("loan-close", receipt.close())
        .emit_coin_amount("prev-margin-interest", receipt.previous_margin_paid())
        .emit_coin_amount("prev-loan-interest", receipt.previous_interest_paid())
        .emit_coin_amount("curr-margin-interest", receipt.current_margin_paid())
        .emit_coin_amount("curr-loan-interest", receipt.current_interest_paid())
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
