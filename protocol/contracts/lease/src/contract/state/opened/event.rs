use dex::MaxSlippage;
use finance::liability::Level;
use platform::batch::{Emit, Emitter};
use sdk::cosmwasm_std::{Addr, Env};

use crate::{
    api::DownpaymentCoin,
    contract::{
        cmd::{OpenLoanRespResult, RepayEmitter},
        state::event as state_event,
    },
    event::Type,
    lease::LeaseDTO,
    loan::RepayReceipt,
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
        .emit_currency_dto("currency", &lease.position.amount().currency())
        .emit("loan-pool-id", lease.loan.lpp().addr())
        .emit_coin_dto("loan", &loan.principal)
        .emit_coin_dto("downpayment", &downpayment)
}

pub(super) struct PaymentEmitter<'env>(&'env Env);
impl<'env> PaymentEmitter<'env> {
    pub fn new(env: &'env Env) -> Self {
        Self(env)
    }
}
impl RepayEmitter for PaymentEmitter<'_> {
    fn emit(self, lease: &Addr, receipt: &RepayReceipt) -> Emitter {
        state_event::emit_payment_int(Type::PaidActive, self.0, lease, receipt)
    }
}

pub(super) fn emit_liquidation_warning(lease: &LeaseDTO, level: &Level) -> Emitter {
    emit_lease(Emitter::of_type(Type::LiquidationWarning), lease)
        .emit_percent_amount("ltv", level.ltv())
        .emit_to_string_value("level", level.ordinal())
}

pub(super) fn emit_slippage_anomaly(lease: &LeaseDTO, max_slippage: MaxSlippage) -> Emitter {
    let emitter = emit_lease(Emitter::of_type(Type::SlippageAnomaly), lease);
    max_slippage.emit(emitter, "max_slippage")
}

fn emit_lease(emitter: Emitter, lease: &LeaseDTO) -> Emitter {
    emitter
        .emit("customer", lease.customer.clone())
        .emit("lease", lease.addr.clone())
        .emit_currency_dto("lease-asset", &lease.position.amount().currency())
}
