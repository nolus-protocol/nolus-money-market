use cosmwasm_std::Deps;
use serde::{Deserialize, Serialize};

use platform::batch::Batch as LocalBatch;
use sdk::{
    cosmwasm_std::{DepsMut, Env, QuerierWrapper},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{opened::RepayTrx, PaymentCoin, StateQuery, StateResponse},
    contract::{
        state::{opened::repay, Controller, Response},
        Lease,
    },
    error::ContractResult,
};

#[derive(Serialize, Deserialize)]
pub struct BuyLpn {
    lease: Lease,
    payment: PaymentCoin,
}

impl BuyLpn {
    pub(super) fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    pub(super) fn enter_state(&self, querier: &QuerierWrapper) -> ContractResult<LocalBatch> {
        let mut swap_trx = self.lease.dex.swap(&self.lease.lease.oracle, querier);
        swap_trx.swap_exact_in(&self.payment, self.lease.lease.loan.lpp().currency())?;
        Ok(swap_trx.into())
    }
}

impl Controller for BuyLpn {
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                todo!("proceed with TransferIn before landing to the same Lease::repay call");
            }
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => todo!(),
        }
    }

    fn query(self, deps: Deps, env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        repay::query(self.lease.lease, self.payment, RepayTrx::Swap, &deps, &env)
    }
}
