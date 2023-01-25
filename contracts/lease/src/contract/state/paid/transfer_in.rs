use cosmwasm_std::{Deps, DepsMut, Env, Timestamp};
use platform::batch::Batch;
use sdk::neutron_sdk::sudo::msg::SudoMsg;
use serde::{Deserialize, Serialize};

use crate::api::paid::ClosingTrx;
use crate::api::{StateQuery, StateResponse};
use crate::contract::state::{Controller, Response};
use crate::contract::Lease;
use crate::error::ContractResult;

#[derive(Serialize, Deserialize)]
pub struct TransferIn {
    lease: Lease,
}

impl TransferIn {
    pub(in crate::contract::state) fn _new(lease: Lease) -> Self {
        Self { lease }
    }

    pub(in crate::contract::state) fn _enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.lease.lease.amount)?;
        Ok(sender.into())
    }
}

impl Controller for TransferIn {
    fn sudo(self, _deps: &mut DepsMut, _env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => todo!("call Lease::close"),
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
