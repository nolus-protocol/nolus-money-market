use cosmwasm_std::{Deps, DepsMut, Env, Timestamp};
use platform::batch::Batch;
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::opened::{OngoingTrx, RepayTrx};
use crate::api::{PaymentCoin, StateQuery, StateResponse};
use crate::contract::cmd::LeaseState;
use crate::contract::state::{Controller, Response};
use crate::contract::Lease;
use crate::error::ContractResult;
use crate::lease::with_lease;

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
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => {
                todo!(
                    "proceed with Swap - TransferIn before landing to the same Lease::repay call"
                );
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
        let in_progress = OngoingTrx::Repayment {
            payment: self.payment,
            in_progress: RepayTrx::TransferOut,
        };

        with_lease::execute(
            self.lease.lease,
            LeaseState::new(env.block.time, Some(in_progress)),
            &env.contract.address,
            &deps.querier,
        )
    }
}
