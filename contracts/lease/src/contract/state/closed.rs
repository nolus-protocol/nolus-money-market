use serde::{Deserialize, Serialize};

use platform::{
    bank,
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Deps, Env, MessageInfo, QuerierWrapper, Timestamp};

use crate::{
    api::StateResponse,
    contract::{cmd::Close, Lease},
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
        lease: Lease,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<MessageResponse> {
        let lease_addr = lease.lease.addr.clone();
        let emitter = self.emit_ok(env, &lease.lease);
        let lease_account = bank::account(&lease_addr, querier);
        let customer = lease.lease.customer.clone();

        with_lease_paid::execute(lease.lease, Close::new(lease_account))
            .and_then(|close_msgs| {
                lease
                    .finalizer
                    .notify(customer)
                    .map(|finalizer_msgs| close_msgs.merge(finalizer_msgs)) //make sure the finalizer messages go out last
            })
            .map(|all_messages| MessageResponse::messages_with_events(all_messages, emitter))
    }

    fn emit_ok(&self, env: &Env, lease: &LeaseDTO) -> Emitter {
        Emitter::of_type(Type::Closed)
            .emit("id", lease.addr.clone())
            .emit_tx_info(env)
    }
}

impl Handler for Closed {
    fn state(
        self,
        _now: Timestamp,
        _querier: &QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        Ok(StateResponse::Closed())
    }

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
