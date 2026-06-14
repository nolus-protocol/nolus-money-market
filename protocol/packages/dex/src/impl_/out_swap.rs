//! A transfer-out-then-remote-swap composite
//!
//! The funding coins are transferred out to the lease's account over the ICA
//! transport, then the asset-to-output swap legs run over the remote-lease
//! controller. Unlike the opening composite there is no ICA-open leg - an
//! opened lease already holds its account. Its terminal `finish` builds and
//! enters a separate drain state.

use serde::{Deserialize, Serialize};

use super::{RemoteSwap, SlippageAnomaly, TransferOut, TransferOutRespDelivery};
use crate::{ForwardToInner, SwapTask as SwapTaskT};

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
))]
pub enum State<SwapTask, SwapClient, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    TransferOut(TransferOut<SwapTask, Self, SwapClient, RemoteSwap<SwapTask, Self>>),
    TransferOutRespDelivery(
        TransferOutRespDelivery<
            SwapTask,
            Self,
            SwapClient,
            ForwardToInnerMsg,
            RemoteSwap<SwapTask, Self>,
        >,
    ),
    RemoteSwap(RemoteSwap<SwapTask, Self>),
    SlippageAnomaly(SlippageAnomaly<SwapTask, Self>),
}

pub type StartOutSwapState<SwapTask, SwapClient, ForwardToInnerMsg> = TransferOut<
    SwapTask,
    State<SwapTask, SwapClient, ForwardToInnerMsg>,
    SwapClient,
    RemoteSwap<SwapTask, State<SwapTask, SwapClient, ForwardToInnerMsg>>,
>;

/// Build the workflow's entry state - the transfer-out leg of the funding coins
pub fn start<SwapTask, SwapClient, ForwardToInnerMsg>(
    spec: SwapTask,
) -> StartOutSwapState<SwapTask, SwapClient, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
    ForwardToInnerMsg: ForwardToInner,
{
    StartOutSwapState::new(spec)
}

mod impl_into {
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{
            ForwardToInner, RemoteSwap, SlippageAnomaly, TransferOut, TransferOutRespDelivery,
        },
    };

    use super::State;

    impl<SwapTask, SwapClient, ForwardToInnerMsg>
        From<TransferOut<SwapTask, Self, SwapClient, RemoteSwap<SwapTask, Self>>>
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: TransferOut<SwapTask, Self, SwapClient, RemoteSwap<SwapTask, Self>>,
        ) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<SwapTask, SwapClient, ForwardToInnerMsg>
        From<
            TransferOutRespDelivery<
                SwapTask,
                Self,
                SwapClient,
                ForwardToInnerMsg,
                RemoteSwap<SwapTask, Self>,
            >,
        > for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(
            value: TransferOutRespDelivery<
                SwapTask,
                Self,
                SwapClient,
                ForwardToInnerMsg,
                RemoteSwap<SwapTask, Self>,
            >,
        ) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<SwapTask, SwapClient, ForwardToInnerMsg> From<RemoteSwap<SwapTask, Self>>
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: RemoteSwap<SwapTask, Self>) -> Self {
            Self::RemoteSwap(value)
        }
    }

    impl<SwapTask, SwapClient, ForwardToInnerMsg> From<SlippageAnomaly<SwapTask, Self>>
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
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
        SwapTask as SwapTaskT,
        impl_::{
            self, ForwardToInner, Handler, RemoteSwapClient,
            response::{ContinueResult, Result},
        },
    };

    use super::State;

    impl<SwapTask, SwapClient, ForwardToInnerMsg> Handler
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT + RemoteSwapClient,
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
                State::TransferOut(inner) => inner.authz_remote_callback(querier, info),
                State::TransferOutRespDelivery(inner) => inner.authz_remote_callback(querier, info),
                State::RemoteSwap(inner) => inner.authz_remote_callback(querier, info),
                State::SlippageAnomaly(inner) => inner.authz_remote_callback(querier, info),
            }
        }

        fn on_open_ica(
            self,
            counterparty_version: String,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::RemoteSwap(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
            }
        }

        fn on_response(
            self,
            response: Binary,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    impl_::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::TransferOutRespDelivery(inner) => {
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
                State::TransferOut(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
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
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::RemoteSwap(inner) => Handler::on_timeout(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::RemoteSwap(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::SlippageAnomaly(inner) => Handler::on_inner(inner, querier, env).map_into(),
            }
        }

        fn on_inner_continue(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner_continue(inner, querier, env),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::RemoteSwap(inner) => Handler::on_inner_continue(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_inner_continue(inner, querier, env),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::heal(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::RemoteSwap(inner) => Handler::heal(inner, querier, env).map_into(),
                State::SlippageAnomaly(inner) => Handler::heal(inner, querier, env).map_into(),
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
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
                State::TransferOut(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
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

        fn on_remote_response(
            self,
            data: Binary,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
            }
        }

        fn on_remote_error(
            self,
            response: ICAErrorResponse,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_remote_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
                State::RemoteSwap(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
            }
        }

        fn price_alarm_dropped(&self) -> Option<Emitter> {
            match self {
                State::TransferOut(inner) => inner.price_alarm_dropped(),
                State::TransferOutRespDelivery(inner) => inner.price_alarm_dropped(),
                State::RemoteSwap(inner) => inner.price_alarm_dropped(),
                State::SlippageAnomaly(inner) => inner.price_alarm_dropped(),
            }
        }
    }
}

mod impl_contract {
    use finance::{duration::Duration, instant::Instant};
    use sdk::cosmwasm_std::QuerierWrapper;

    use crate::{
        Contract, ContractInRemoteSwap, ContractInSwap, ForwardToInner, SwapTask as SwapTaskT,
    };

    use super::State;

    impl<SwapTask, SwapClient, ForwardToInnerMsg> Contract
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT
            + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>
            + ContractInRemoteSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
        ForwardToInnerMsg: ForwardToInner,
    {
        type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

        fn state(
            self,
            now: Instant,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::TransferOut(inner) => Contract::state(inner, now, due_projection, querier),
                State::TransferOutRespDelivery(inner) => {
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
    use std::fmt::{Display, Formatter, Result as FmtResult};

    use crate::SwapTask as SwapTaskT;

    use super::State;

    impl<SwapTask, SwapClient, ForwardToInnerMsg> Display
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                State::TransferOut(inner) => Display::fmt(inner, f),
                State::TransferOutRespDelivery(inner) => Display::fmt(inner, f),
                State::RemoteSwap(inner) => inner.fmt(f),
                State::SlippageAnomaly(inner) => inner.fmt(f),
            }
        }
    }
}

#[cfg(feature = "migration")]
mod impl_migration {
    use super::{super::migration::InspectSpec, State};
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{ForwardToInner, migration::MigrateSpec},
    };

    impl<SwapTask, SwapTaskNew, SwapClient, ForwardToInnerMsg>
        MigrateSpec<SwapTask, SwapTaskNew, State<SwapTaskNew, SwapClient, ForwardToInnerMsg>>
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
        SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
    {
        type Out = State<SwapTaskNew, SwapClient, ForwardToInnerMsg>;

        fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
        where
            MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
        {
            match self {
                State::TransferOut(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
                State::RemoteSwap(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SlippageAnomaly(inner) => inner.migrate_spec(migrate_fn).into(),
            }
        }
    }

    impl<SwapTask, R, SwapClient, ForwardToInnerMsg> InspectSpec<SwapTask, R>
        for State<SwapTask, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
        where
            InspectFn: FnOnce(&SwapTask) -> R,
        {
            match self {
                State::TransferOut(inner) => inner.inspect_spec(inspect_fn),
                State::TransferOutRespDelivery(inner) => inner.inspect_spec(inspect_fn),
                State::RemoteSwap(inner) => inner.inspect_spec(inspect_fn),
                State::SlippageAnomaly(inner) => inner.inspect_spec(inspect_fn),
            }
        }
    }
}
