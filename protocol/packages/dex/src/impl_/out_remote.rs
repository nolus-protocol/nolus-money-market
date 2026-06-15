use serde::{Deserialize, Serialize};

use crate::{
    Connectable, IcaConnectee, SwapTask as SwapTaskT,
    impl_::{
        IcaConnector, RemoteSwap, SlippageAnomaly, TransferOut, TransferOutRespDelivery,
        resp_delivery::ICAOpenResponseDelivery,
    },
};

pub type OpenIcaRespDelivery<OpenIca, SwapResult, ForwardToInnerMsg> =
    ICAOpenResponseDelivery<IcaConnector<OpenIca, SwapResult>, ForwardToInnerMsg>;

/// The composite of the remote-lease opening workflow
///
/// The ICA account and the transfer-out legs run over the ICA transport;
/// the swap leg runs over the remote-lease controller. There is no local
/// DEX swap leg in this composite.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "OpenIca: Serialize, SwapTask: Serialize",
    deserialize = "OpenIca: Deserialize<'de>, SwapTask: Deserialize<'de>",
))]
pub enum State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
where
    SwapTask: SwapTaskT,
{
    OpenIca(IcaConnector<OpenIca, SwapTask::Result>),
    OpenIcaRespDelivery(OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>),
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
    /// Present only to satisfy the `RemoteSwap` transport's terminal bound -
    /// the opening swap re-emits on a slippage anomaly and never parks, so a
    /// follow-up issue owns the opening-leg terminal (see issue #655).
    SlippageAnomaly(SlippageAnomaly<SwapTask, Self>),
}

pub type StartLocalRemoteState<OpenIca, SwapTask> =
    IcaConnector<OpenIca, <SwapTask as SwapTaskT>::Result>;

pub fn start<OpenIca, SwapTask>(connectee: OpenIca) -> StartLocalRemoteState<OpenIca, SwapTask>
where
    OpenIca: IcaConnectee + Connectable,
    SwapTask: SwapTaskT,
{
    StartLocalRemoteState::<OpenIca, SwapTask>::new(connectee)
}

mod impl_into {
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{IcaConnector, RemoteSwap, SlippageAnomaly, TransferOut, TransferOutRespDelivery},
    };

    use super::{OpenIcaRespDelivery, State};

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<IcaConnector<OpenIca, SwapTask::Result>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: IcaConnector<OpenIca, SwapTask::Result>) -> Self {
            Self::OpenIca(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>,
        ) -> Self {
            Self::OpenIcaRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOut<SwapTask, Self, SwapClient, RemoteSwap<SwapTask, Self>>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: TransferOut<SwapTask, Self, SwapClient, RemoteSwap<SwapTask, Self>>,
        ) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<
            TransferOutRespDelivery<
                SwapTask,
                Self,
                SwapClient,
                ForwardToInnerMsg,
                RemoteSwap<SwapTask, Self>,
            >,
        > for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
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

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<RemoteSwap<SwapTask, Self>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: RemoteSwap<SwapTask, Self>) -> Self {
            Self::RemoteSwap(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SlippageAnomaly<SwapTask, Self>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SlippageAnomaly<SwapTask, Self>) -> Self {
            Self::SlippageAnomaly(value)
        }
    }
}

mod impl_handler {
    use std::fmt::Display;

    use platform::{batch::Emitter, ica::ErrorResponse as ICAErrorResponse};
    use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

    use crate::{
        Connectable, IcaConnectee, SwapTask as SwapTaskT, TimeAlarm,
        impl_::{
            self, ForwardToInner, Handler, RemoteSwapClient,
            response::{ContinueResult, Result},
        },
    };

    use super::State;

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Connectable + IcaConnectee<State = Self> + TimeAlarm + Display,
        SwapTask: SwapTaskT + RemoteSwapClient,
        ForwardToInnerMsg: ForwardToInner,
        ForwardToInnerContinueMsg: ForwardToInner,
    {
        type Response = Self;
        type SwapResult = SwapTask::Result;

        fn authz_remote_callback(
            &self,
            querier: QuerierWrapper<'_>,
            info: &MessageInfo,
        ) -> crate::error::Result<()> {
            match self {
                State::OpenIca(inner) => inner.authz_remote_callback(querier, info),
                State::OpenIcaRespDelivery(inner) => inner.authz_remote_callback(querier, info),
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
                State::OpenIca(inner) => impl_::forward_to_inner_ica::<
                    _,
                    ForwardToInnerContinueMsg,
                    Self,
                >(inner, counterparty_version, env),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
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
                State::OpenIca(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
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
                State::OpenIca(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
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
                State::OpenIca(inner) => Handler::on_timeout(inner, querier, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::RemoteSwap(inner) => Handler::on_timeout(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
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
                State::OpenIca(inner) => Handler::on_inner_continue(inner, querier, env),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::TransferOut(inner) => Handler::on_inner_continue(inner, querier, env),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::RemoteSwap(inner) => Handler::on_inner_continue(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_inner_continue(inner, querier, env),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env, info: &MessageInfo) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::heal(inner, querier, env, info).map_into(),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::heal(inner, querier, env, info).map_into()
                }
                State::TransferOut(inner) => Handler::heal(inner, querier, env, info).map_into(),
                State::TransferOutRespDelivery(inner) => {
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
                State::OpenIca(inner) => Handler::reply(inner, querier, env, msg),
                State::OpenIcaRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
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
                State::OpenIca(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
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

        /// Remote-lease controller callbacks reach only the leg that
        /// scheduled a remote operation - the swap leg. Every other leg
        /// absorbs them with an event via the [`Handler`] defaults: an
        /// `Err` would revert the controller's acknowledgment transaction
        /// and strand the relayer, while routing into the leg's ICA entry
        /// points would silently advance its acknowledgment countdown.
        fn on_remote_response(
            self,
            data: Binary,
            nonce: u64,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::OpenIca(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::TransferOut(inner) => {
                    Handler::on_remote_response(inner, data, nonce, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
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
                State::OpenIca(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::TransferOut(inner) => {
                    Handler::on_remote_error(inner, response, nonce, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
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
                State::OpenIca(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::TransferOut(inner) => {
                    Handler::on_remote_timeout(inner, nonce, querier, env).map_into()
                }
                State::TransferOutRespDelivery(inner) => {
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
                State::OpenIca(inner) => inner.price_alarm_dropped(),
                State::OpenIcaRespDelivery(inner) => inner.price_alarm_dropped(),
                State::TransferOut(inner) => inner.price_alarm_dropped(),
                State::TransferOutRespDelivery(inner) => inner.price_alarm_dropped(),
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

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Contract
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Contract,
        SwapTask: SwapTaskT<StateResponse = OpenIca::StateResponse>
            + ContractInSwap<StateResponse = OpenIca::StateResponse>
            + ContractInRemoteSwap<StateResponse = OpenIca::StateResponse>,
    {
        type StateResponse = OpenIca::StateResponse;

        fn state(
            self,
            now: Instant,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::OpenIca(inner) => Contract::state(inner, now, due_projection, querier),
                State::OpenIcaRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
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
    use std::fmt::Display;

    use crate::SwapTask as SwapTaskT;

    use super::State;

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Display
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Display,
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                State::OpenIca(inner) => Display::fmt(inner, f),
                State::OpenIcaRespDelivery(inner) => Display::fmt(inner, f),
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

    use super::{OpenIcaRespDelivery, State};
    use crate::{
        Connectable, IcaConnectee, SwapTask as SwapTaskT,
        impl_::{ForwardToInner, IcaConnector, migration::MigrateSpec},
    };

    //cannot impl MigrateSpec due to the need to migrate OpenIca as well
    impl<SwapTask, OpenIca, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        pub fn migrate<MigrateOpenIcaFn, MigrateSpecFn, OpenIcaNew, SwapTaskNew>(
            self,
            migrate_open_ica: MigrateOpenIcaFn,
            migrate_spec: MigrateSpecFn,
        ) -> State<OpenIcaNew, SwapTaskNew, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        where
            OpenIca: MigrateSpec<
                    OpenIca,
                    OpenIcaNew,
                    State<
                        OpenIcaNew,
                        SwapTaskNew,
                        SwapClient,
                        ForwardToInnerMsg,
                        ForwardToInnerContinueMsg,
                    >,
                >,
            OpenIca::Out: IcaConnectee + Connectable,
            IcaConnector<OpenIca::Out, SwapTask::Result>: Into<
                State<
                    OpenIcaNew,
                    SwapTaskNew,
                    SwapClient,
                    ForwardToInnerMsg,
                    ForwardToInnerContinueMsg,
                >,
            >,
            OpenIcaRespDelivery<OpenIca::Out, SwapTask::Result, ForwardToInnerContinueMsg>: Into<
                State<
                    OpenIcaNew,
                    SwapTaskNew,
                    SwapClient,
                    ForwardToInnerMsg,
                    ForwardToInnerContinueMsg,
                >,
            >,
            MigrateOpenIcaFn: FnOnce(OpenIca) -> OpenIcaNew,
            MigrateSpecFn: FnOnce(SwapTask) -> SwapTaskNew,
            SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
        {
            match self {
                State::OpenIca(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::OpenIcaRespDelivery(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::TransferOut(inner) => inner.migrate_spec(migrate_spec).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_spec).into(),
                State::RemoteSwap(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SlippageAnomaly(inner) => inner.migrate_spec(migrate_spec).into(),
            }
        }
    }
}
