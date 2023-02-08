use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo, QuerierWrapper};
use platform::batch::{Emit, Emitter};
use serde::{Deserialize, Serialize};

use crate::api::paid::ClosingTrx;
use crate::api::{ExecuteMsg, StateQuery, StateResponse};
use crate::contract::state::closed::Closed;
use crate::contract::state::transfer_in;
use crate::contract::state::{Controller, Response};
use crate::contract::{state, Lease};
use crate::error::ContractResult;
use crate::event::Type;

use super::transfer_in_init::TransferInInit;

#[derive(Serialize, Deserialize)]
pub struct TransferInFinish {
    lease: Lease,
}

impl TransferInFinish {
    pub(super) fn try_complete(
        self,
        querier: &QuerierWrapper,
        env: &Env,
    ) -> ContractResult<Response> {
        let received =
            transfer_in::check_received(&self.lease.lease.amount, &env.contract.address, querier)?;

        if received {
            Closed::default().enter_state(self.lease.lease, env, querier)
        } else {
            let emitter = self.emit_ok();
            let batch =
                transfer_in::setup_alarm(self.lease.lease.time_alarms.clone(), env.block.time)?;
            Ok(Response::from(batch.into_response(emitter), self))
        }
    }

    fn on_alarm(self, querier: &QuerierWrapper, env: &Env) -> ContractResult<Response> {
        self.try_complete(querier, env)
    }

    fn emit_ok(&self) -> Emitter {
        Emitter::of_type(Type::ClosingTransferIn)
            .emit("id", self.lease.lease.addr.clone())
            .emit_coin_dto("payment", self.lease.lease.amount.clone())
    }
}

impl From<TransferInInit> for TransferInFinish {
    fn from(init: TransferInInit) -> Self {
        Self { lease: init.lease }
    }
}

impl Controller for TransferInFinish {
    fn execute(
        self,
        deps: &mut DepsMut,
        env: Env,
        _info: MessageInfo,
        msg: ExecuteMsg,
    ) -> ContractResult<Response> {
        if matches!(msg, ExecuteMsg::TimeAlarm {}) {
            self.on_alarm(&deps.querier, &env)
        } else {
            state::err(&format!("{:?}", msg))
        }
    }

    fn query(self, _deps: Deps, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(StateResponse::Paid {
            amount: self.lease.lease.amount,
            in_progress: Some(ClosingTrx::TransferInFinish),
        })
    }
}
