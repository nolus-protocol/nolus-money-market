use serde::{Deserialize, Serialize};

use crate::{
    SwapTask as SwapTaskT,
    error::Result as DexResult,
    impl_::{Funding, FundingClient, FundingRespDelivery, RemoteSwap, SlippageAnomaly},
};

/// The composite of the remote-lease opening workflow
///
/// The funding legs transfer the downpayment and the principal to the lease's
/// Solana-side `LeaseAuthority` over the paired ICS-20 transfer channel; the
/// swap leg runs over the remote-lease controller. There is neither an
/// Interchain Account nor a local DEX swap leg in this composite.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de> + SwapTaskT",
))]
pub enum State<SwapTask, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    Funding(Funding<SwapTask, Self, RemoteSwap<SwapTask, Self>>),
    FundingRespDelivery(
        FundingRespDelivery<SwapTask, Self, ForwardToInnerMsg, RemoteSwap<SwapTask, Self>>,
    ),
    RemoteSwap(RemoteSwap<SwapTask, Self>),
    /// Present only to satisfy the `RemoteSwap` transport's terminal bound -
    /// the opening swap re-emits on a slippage anomaly and never parks, so a
    /// follow-up issue owns the opening-leg terminal (see issue #655).
    SlippageAnomaly(SlippageAnomaly<SwapTask, Self>),
}

pub type StartFundRemoteState<SwapTask, ForwardToInnerMsg> = Funding<
    SwapTask,
    State<SwapTask, ForwardToInnerMsg>,
    RemoteSwap<SwapTask, State<SwapTask, ForwardToInnerMsg>>,
>;

pub fn start<SwapTask, ForwardToInnerMsg>(
    spec: SwapTask,
) -> DexResult<StartFundRemoteState<SwapTask, ForwardToInnerMsg>>
where
    SwapTask: SwapTaskT + FundingClient,
{
    Funding::start(spec)
}

mod impl_into {
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{Funding, FundingRespDelivery, RemoteSwap, SlippageAnomaly},
    };

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> From<Funding<SwapTask, Self, RemoteSwap<SwapTask, Self>>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: Funding<SwapTask, Self, RemoteSwap<SwapTask, Self>>) -> Self {
            Self::Funding(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg>
        From<FundingRespDelivery<SwapTask, Self, ForwardToInnerMsg, RemoteSwap<SwapTask, Self>>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: FundingRespDelivery<
                SwapTask,
                Self,
                ForwardToInnerMsg,
                RemoteSwap<SwapTask, Self>,
            >,
        ) -> Self {
            Self::FundingRespDelivery(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<RemoteSwap<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: RemoteSwap<SwapTask, Self>) -> Self {
            Self::RemoteSwap(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<SlippageAnomaly<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SlippageAnomaly<SwapTask, Self>) -> Self {
            Self::SlippageAnomaly(value)
        }
    }
}

mod impl_handler {
    use platform::{batch::Emitter, ica::ErrorResponse as ICAErrorResponse};
    use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

    use crate::{
        FundingClient, SwapTask as SwapTaskT,
        impl_::{
            self, ForwardToInner, Handler, RemoteSwapClient,
            response::{ContinueResult, Result},
        },
    };

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> Handler for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT + RemoteSwapClient + FundingClient,
        ForwardToInnerMsg: ForwardToInner,
    {
        type Response = Self;
        type SwapResult = SwapTask::Result;

        fn authz_remote_callback(
            &self,
            querier: QuerierWrapper<'_>,
            info: &MessageInfo,
        ) -> crate::error::Result<()> {
            match self {
                State::Funding(inner) => inner.authz_remote_callback(querier, info),
                State::FundingRespDelivery(inner) => inner.authz_remote_callback(querier, info),
                State::RemoteSwap(inner) => inner.authz_remote_callback(querier, info),
                State::SlippageAnomaly(inner) => inner.authz_remote_callback(querier, info),
            }
        }

        fn on_response(
            self,
            response: Binary,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    impl_::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_error(
            self,
            response: ICAErrorResponse,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::Funding(inner) => Handler::on_timeout(inner, querier, env),
                State::FundingRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::RemoteSwap(inner) => Handler::on_timeout(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::Funding(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::FundingRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::RemoteSwap(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::SlippageAnomaly(inner) => Handler::on_inner(inner, querier, env).map_into(),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env, info: &MessageInfo) -> Result<Self> {
            match self {
                State::Funding(inner) => Handler::heal(inner, querier, env, info).map_into(),
                State::FundingRespDelivery(inner) => {
                    Handler::heal(inner, querier, env, info).map_into()
                }
                State::RemoteSwap(inner) => Handler::heal(inner, querier, env, info).map_into(),
                State::SlippageAnomaly(inner) => {
                    Handler::heal(inner, querier, env, info).map_into()
                }
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::Funding(inner) => Handler::reply(inner, querier, env, msg),
                State::FundingRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::RemoteSwap(inner) => Handler::reply(inner, querier, env, msg),
                State::SlippageAnomaly(inner) => Handler::reply(inner, querier, env, msg),
            }
        }

        fn on_time_alarm(
            self,
            querier: QuerierWrapper<'_>,
            env: Env,
            info: MessageInfo,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
            }
        }

        /// Remote-lease controller callbacks reach only the leg that scheduled
        /// a remote operation - the swap leg. Every other leg absorbs them via
        /// the [`Handler`] defaults.
        fn on_remote_response(
            self,
            data: Binary,
            nonce: u64,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
            }
        }

        fn on_remote_error(
            self,
            response: ICAErrorResponse,
            nonce: u64,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
            }
        }

        fn on_remote_timeout(
            self,
            nonce: u64,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::Funding(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::FundingRespDelivery(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
            }
        }

        fn price_alarm_dropped(&self) -> Option<Emitter> {
            match self {
                State::Funding(inner) => inner.price_alarm_dropped(),
                State::FundingRespDelivery(inner) => inner.price_alarm_dropped(),
                State::RemoteSwap(inner) => inner.price_alarm_dropped(),
                State::SlippageAnomaly(inner) => inner.price_alarm_dropped(),
            }
        }
    }
}

mod impl_contract {
    use finance::duration::Duration;
    use finance::instant::Instant;
    use sdk::cosmwasm_std::QuerierWrapper;

    use crate::{Contract, ContractInRemoteSwap, ContractInSwap, SwapTask as SwapTaskT};

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> Contract for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT
            + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>
            + ContractInRemoteSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

        fn state(
            self,
            now: Instant,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::Funding(inner) => Contract::state(inner, now, due_projection, querier),
                State::FundingRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::RemoteSwap(inner) => Contract::state(inner, now, due_projection, querier),
                State::SlippageAnomaly(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
            }
        }
    }
}

mod impl_display {
    use std::fmt::Display;

    use crate::SwapTask as SwapTaskT;

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> Display for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                State::Funding(inner) => inner.fmt(f),
                State::FundingRespDelivery(inner) => Display::fmt(inner, f),
                State::RemoteSwap(inner) => inner.fmt(f),
                State::SlippageAnomaly(inner) => inner.fmt(f),
            }
        }
    }
}
