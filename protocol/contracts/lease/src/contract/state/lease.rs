use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use finance::instant::Instant;
use remote_lease::callback::RemoteLeaseCallback;
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Reply};

use crate::{
    api::{
        LeasePaymentCurrencies,
        position::{ClosePolicyChange, PositionClose},
        query::StateResponse,
    },
    error::ContractResult,
};

use super::{
    Contract, Response, State as ContractState, handler::Handler as LeaseHandler, ignore_msg,
};

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct State<H> {
    handler: H,
}

impl<H> State<H> {
    pub fn new(handler: H) -> Self {
        Self { handler }
    }
}

impl<H> Contract for State<H>
where
    H: LeaseHandler,
    Self: Into<ContractState>,
{
    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<StateResponse> {
        self.handler.state(now, due_projection, querier)
    }

    fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContractResult<Response> {
        self.handler.reply(querier, env, msg)
    }

    fn repay(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.repay(querier, env, info)
    }

    fn change_close_policy(
        self,
        change: ClosePolicyChange,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.change_close_policy(change, querier, env, info)
    }

    fn close_position(
        self,
        spec: PositionClose,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.close_position(spec, querier, env, info)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_time_alarm(querier, env, info)
    }

    fn on_price_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.on_price_alarm(querier, env, info)
    }

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> ContractResult<Response> {
        self.handler.heal(querier, env, info)
    }

    /// Absorbs a stale remote-lease callback
    ///
    /// A Lease with a remote operation in flight sits in a [`super::dex::State`],
    /// and the one opening state that owns a live packet, `OpenLease`, is not
    /// wrapped here either. So no `H` this wrapper carries can be awaiting a
    /// callback, which makes every callback reaching it a redelivery of an
    /// operation that already resolved.
    ///
    /// The remote-lease channel is UNORDERED, so such a redelivery is expected
    /// rather than exceptional. Erring would revert the controller's
    /// `ibc_packet_ack` and leave the relayer retrying that packet forever, so
    /// drop it and commit instead.
    fn on_remote_lease_callback(
        self,
        _callback: RemoteLeaseCallback<LeasePaymentCurrencies>,
        _info: MessageInfo,
        _querier: QuerierWrapper<'_>,
        _env: Env,
    ) -> ContractResult<Response> {
        ignore_msg(self)
    }
}
