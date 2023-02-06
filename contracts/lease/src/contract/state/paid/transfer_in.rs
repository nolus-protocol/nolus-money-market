use cosmwasm_std::{Deps, DepsMut, Env, QuerierWrapper, Timestamp};
use platform::batch::{Batch, Emit, Emitter};
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::paid::ClosingTrx;
use crate::api::{StateQuery, StateResponse};
use crate::contract::state::closed::Closed;
use crate::contract::state::{Controller, Response};
use crate::contract::Lease;
use crate::error::ContractResult;
use crate::event::Type;

#[derive(Serialize, Deserialize)]
pub struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.lease.lease.amount)?;
        Ok(sender.into())
    }

    fn on_response(self, env: &Env, querier: &QuerierWrapper) -> ContractResult<Response> {
        let next_state = Closed::default();
        let batch = next_state.enter_state(self.lease.lease, env, querier)?;
        let emitter = Emitter::of_type(Type::Close)
            .emit("id", env.contract.address.clone())
            .emit_tx_info(env);

        Ok(Response::from(batch.into_response(emitter), next_state))
    }
}

impl Controller for TransferIn {
    fn sudo(self, deps: &mut DepsMut, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => self.on_response(&env, &deps.querier),
            SudoMsg::Timeout { request: _ } => todo!(),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: Some(ClosingTrx::TransferIn),
        })
    }
}
