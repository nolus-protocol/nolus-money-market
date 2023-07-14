use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use finance::duration::Duration;
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Addr, Binary, Deps, DepsMut, Env, QuerierWrapper, Reply, Timestamp};

use serde::{Deserialize, Serialize};

use crate::{
    error::Result as DexResult, response::Result, ContinueResult, Contract, ForwardToInner,
    Handler, TimeAlarm,
};

use super::Response;

const REPLY_ID: u64 = 12345678901;

/// Provides guaranteed response delivery
///
/// If the first delivery fails the `ResponseDelivery` leverages the time alarms' guaranteed delivery
/// scheduling a time alarm to make a delivery attempt on the next alarms dispatch cycle.
#[derive(Serialize, Deserialize)]
pub struct ResponseDelivery<H, ForwardToInnerMsg> {
    handler: H,
    response: Binary,
    _forward_to_inner_msg: PhantomData<ForwardToInnerMsg>,
}

impl<H, ForwardToInnerMsg> ResponseDelivery<H, ForwardToInnerMsg> {
    pub fn new(handler: H, response: Binary) -> Self {
        Self {
            handler,
            response,
            _forward_to_inner_msg: PhantomData::default(),
        }
    }
}

impl<H, ForwardToInnerMsg> ResponseDelivery<H, ForwardToInnerMsg>
where
    ForwardToInnerMsg: ForwardToInner,
{
    pub fn enter(&self, myself: Addr) -> DexResult<Batch> {
        // Limitations:
        // 1. Cannot, if ever we want, handle reply_on_success since a successful delivery would have moved to another state
        // 2. Do not support reply_* to the sub-messages
        let mut resp = Batch::default();
        resp.schedule_execute_wasm_reply_error_no_funds(&myself, ForwardToInnerMsg::msg(), REPLY_ID)
            .map(|()| resp)
            .map_err(Into::into)
    }
}

impl<H, ForwardToInnerMsg> ResponseDelivery<H, ForwardToInnerMsg>
where
    H: Handler + TimeAlarm,
    Self: Into<H::Response>,
{
    const RIGHT_AFTER_NOW: Duration = Duration::from_nanos(1);

    fn do_deliver(self, deps: Deps<'_>, env: Env) -> Result<Self> {
        self.handler
            .on_response(self.response, deps, env)
            .map_into()
    }

    fn setup_next_delivery(self, now: Timestamp) -> ContinueResult<Self> {
        self.handler
            .setup_alarm(now + Self::RIGHT_AFTER_NOW)
            .map(|msgs| {
                MessageResponse::messages_with_events(msgs, self.emit_setup_next_delivery())
            })
            .map(|msg_response| Response::<Self>::from(msg_response, self))
    }

    fn emit_setup_next_delivery(&self) -> Emitter {
        Emitter::of_type("next-delivery").emit("what", "dex-response")
    }
}

impl<H, ForwardToInnerMsg> Handler for ResponseDelivery<H, ForwardToInnerMsg>
where
    H: Handler + TimeAlarm,
    Self: Into<H::Response>,
{
    type Response = H::Response;
    type SwapResult = H::SwapResult;

    fn on_inner(self, deps: Deps<'_>, env: Env) -> Result<Self> {
        // the errors from the response delivery herebelow and from a sub-message would be
        // reported in the `fn reply`
        self.do_deliver(deps, env)
    }

    fn reply(self, _deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
        debug_assert_eq!(msg.id, REPLY_ID);
        debug_assert!(msg.result.is_err());

        self.setup_next_delivery(env.block.time)
    }

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
        // we leave the error to escape since the time alarms delivery is reliable
        self.do_deliver(deps, env)
    }
}

impl<H, ForwardToInnerMsg> Contract for ResponseDelivery<H, ForwardToInnerMsg>
where
    H: Contract,
{
    type StateResponse = H::StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        self.handler.state(now, querier)
    }
}

impl<H, ForwardToInnerMsg> Display for ResponseDelivery<H, ForwardToInnerMsg>
where
    H: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("ResponseDelivery({})", self.handler))
    }
}
