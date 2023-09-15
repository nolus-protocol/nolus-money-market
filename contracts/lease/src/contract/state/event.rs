use currency::Currency;
use finance::liability::Cause;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::{
    api::LeaseCoin,
    contract::cmd::{LiquidationDTO, RepayEmitter},
    event::Type,
    loan::RepayReceipt,
};

pub(super) struct LiquidationEmitter<'env> {
    liquidation: LiquidationDTO,
    liquidation_amount: LeaseCoin,
    env: &'env Env,
}

impl<'env> LiquidationEmitter<'env> {
    pub fn new(liquidation: LiquidationDTO, liquidation_amount: LeaseCoin, env: &'env Env) -> Self {
        Self {
            liquidation,
            liquidation_amount,
            env,
        }
    }
}
impl<'env> RepayEmitter for LiquidationEmitter<'env> {
    fn emit<Lpn>(self, lease: &Addr, receipt: &RepayReceipt<Lpn>) -> Emitter
    where
        Lpn: Currency,
    {
        let emitter = emit_payment_int(Type::Liquidation, self.env, lease, receipt);
        emit_liquidation_info(emitter, self.liquidation.cause(), &self.liquidation_amount)
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
