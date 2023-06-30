use serde::{Deserialize, Serialize};

use platform::{
    bank,
    batch::{Batch, Emit, Emitter},
};
use sdk::cosmwasm_std::{Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::StateResponse,
    contract::{cmd::Close, Contract},
    error::ContractResult,
    event::Type,
    lease::{with_lease_paid, LeaseDTO},
};

use super::{Handler, Response};

#[derive(Serialize, Deserialize, Default)]
pub struct Closed {}

impl Closed {
    pub(super) fn enter_state(
        &self,
        lease: LeaseDTO,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Batch> {
        let lease_addr = lease.addr.clone();
        let lease_account = bank::account(&lease_addr, querier);
        with_lease_paid::execute(lease, Close::new(lease_account))
    }

    pub(super) fn emit_ok(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl Handler for Closed {
    fn on_time_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
    fn on_price_alarm(
        self,
        _deps: Deps<'_>,
        _env: Env,
        _info: MessageInfo,
    ) -> ContractResult<Response> {
        super::ignore_msg(self)
    }
}

impl Contract for Closed {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }
}
