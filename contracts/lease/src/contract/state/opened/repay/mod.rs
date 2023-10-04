use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::LpnCoin,
    contract::{cmd::RepayLeaseFn, state::Response, Lease},
    error::ContractResult,
};

use super::{
    event::PaymentEmitter,
    payment::{Repay, RepayAlgo, Repayable},
};

pub mod buy_lpn;
#[cfg(feature = "migration")]
pub(in crate::contract::state) mod v5;

pub(super) fn repay(
    lease: Lease,
    amount: LpnCoin,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response> {
    Repay::from(CustomerRepay {}).try_repay(lease, amount, env, querier)
}

pub(super) struct CustomerRepay {}

impl RepayAlgo for CustomerRepay {
    type RepayFn = RepayLeaseFn;

    type PaymentEmitter<'liq, 'env> = PaymentEmitter<'env>;

    fn repay_fn(&self) -> Self::RepayFn {
        Self::RepayFn {}
    }

    fn emitter_fn<'liq, 'env>(&'liq self, env: &'env Env) -> Self::PaymentEmitter<'liq, 'env> {
        Self::PaymentEmitter::new(env)
    }
}
