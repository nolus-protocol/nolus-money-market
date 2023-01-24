use cosmwasm_std::{Deps, DepsMut, Env, Timestamp};
use platform::batch::Batch;
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::opened::RepayTrx;
use crate::api::{PaymentCoin, StateQuery, StateResponse};
use crate::contract::state::{opened::repay, Controller, Response};
use crate::contract::Lease;
use crate::error::ContractResult;

use super::buy_lpn::BuyLpn;

#[derive(Serialize, Deserialize)]
pub struct TransferOut {
    lease: Lease,
    payment: PaymentCoin,
}

impl TransferOut {
    pub(in crate::contract::state::opened) fn new(lease: Lease, payment: PaymentCoin) -> Self {
        Self { lease, payment }
    }

    pub(in crate::contract::state::opened) fn enter_state(
        &self,
        now: Timestamp,
    ) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_to(now);
        // TODO apply nls_swap_fee on the payment!
        sender.send(&self.payment)?;
        Ok(sender.into())
    }
}

impl Controller for TransferOut {
    fn sudo(self, deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                let next_state = BuyLpn::new(self.lease, self.payment);
                let batch = next_state.enter_state(&deps.querier)?;
                Ok(Response::from(batch, next_state))
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
        repay::query(
            self.lease.lease,
            self.payment,
            RepayTrx::TransferOut,
            &deps,
            &env,
        )
    }
}
