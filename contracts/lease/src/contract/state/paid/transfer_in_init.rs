use serde::{Deserialize, Serialize};

use platform::batch::Batch;
use sdk::{
    cosmwasm_std::{Deps, DepsMut, Env, QuerierWrapper, Timestamp},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{dex::ConnectionParams, paid::ClosingTrx, StateQuery, StateResponse},
    contract::{
        dex::DexConnectable,
        state::{self, Controller, Response},
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use super::transfer_in_finish::TransferInFinish;

#[derive(Serialize, Deserialize)]
pub struct TransferInInit {
    pub(super) lease: Lease,
}

impl TransferInInit {
    pub(in crate::contract::state) fn new(lease: Lease) -> Self {
        Self { lease }
    }

    fn enter_state(&self, now: Timestamp) -> ContractResult<Batch> {
        let mut sender = self.lease.dex.transfer_from(now);
        sender.send(&self.lease.lease.amount)?;
        Ok(sender.into())
    }

    fn on_response(self, env: &Env, querier: &QuerierWrapper<'_>) -> ContractResult<Response> {
        TransferInFinish::from(self).try_complete(querier, env)
    }
}

impl DexConnectable for TransferInInit {
    fn dex(&self) -> &ConnectionParams {
        self.lease.dex()
    }
}

impl Controller for TransferInInit {
    fn enter(&self, _deps: Deps<'_>, env: Env) -> ContractResult<Batch> {
        self.enter_state(env.block.time)
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::Response {
                request: _,
                data: _,
            } => self.on_response(&env, &deps.querier),
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_repair_channel(self, Type::ClosingTransferIn, deps, env)
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: Some(ClosingTrx::TransferInInit),
        })
    }
}
