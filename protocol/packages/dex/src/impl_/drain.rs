//! The composite of the remote-account drain workflow
//!
//! The coins are transferred out of the remote account over the
//! remote-lease controller, one in-flight transfer at a time, then their
//! arrival on the local account is awaited. There is no ICA leg in this
//! composite.

use serde::{Deserialize, Serialize};

use crate::impl_::{
    funds_arrival::FundsArrival,
    remote_transfer_out::{RemoteTransferOut, RemoteTransferOutTask},
};

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "Task: Serialize",
    deserialize = "Task: Deserialize<'de> + RemoteTransferOutTask"
))]
pub enum State<Task>
where
    Task: RemoteTransferOutTask,
{
    TransferOut(RemoteTransferOut<Task, Self>),
    FundsArrival(FundsArrival<Task, Self>),
}

pub type StartDrainState<Task> = RemoteTransferOut<Task, State<Task>>;

/// Build the workflow's entry state over the task
///
/// Errs if the task holds no coins or more than the supported maximum.
/// The first transfer goes out on [`Enterable::enter`][crate::Enterable].
pub fn start<Task>(task: Task) -> crate::error::Result<StartDrainState<Task>>
where
    Task: RemoteTransferOutTask,
{
    RemoteTransferOut::start(task)
}

mod impl_into {
    use crate::impl_::{
        funds_arrival::FundsArrival,
        remote_transfer_out::{RemoteTransferOut, RemoteTransferOutTask},
    };

    use super::State;

    impl<Task> From<RemoteTransferOut<Task, Self>> for State<Task>
    where
        Task: RemoteTransferOutTask,
    {
        fn from(value: RemoteTransferOut<Task, Self>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<Task> From<FundsArrival<Task, Self>> for State<Task>
    where
        Task: RemoteTransferOutTask,
    {
        fn from(value: FundsArrival<Task, Self>) -> Self {
            Self::FundsArrival(value)
        }
    }
}

mod impl_handler {
    use platform::ica::ErrorResponse as ICAErrorResponse;
    use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

    use crate::{
        error::Result as DexResult,
        impl_::{
            remote_transfer_out::RemoteTransferOutTask,
            response::{ContinueResult, Handler, Result},
        },
    };

    use super::State;

    impl<Task> Handler for State<Task>
    where
        Task: RemoteTransferOutTask,
    {
        type Response = Self;
        type SwapResult = Task::Result;

        fn authz_remote_callback(
            &self,
            querier: QuerierWrapper<'_>,
            info: &MessageInfo,
        ) -> DexResult<()> {
            match self {
                State::TransferOut(inner) => inner.authz_remote_callback(querier, info),
                State::FundsArrival(inner) => inner.authz_remote_callback(querier, info),
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
                State::FundsArrival(inner) => {
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
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::FundsArrival(inner) => {
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
                State::FundsArrival(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::FundsArrival(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::FundsArrival(inner) => Handler::on_inner(inner, querier, env).map_into(),
            }
        }

        fn on_inner_continue(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner_continue(inner, querier, env),
                State::FundsArrival(inner) => Handler::on_inner_continue(inner, querier, env),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env, info: &MessageInfo) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::heal(inner, querier, env, info).map_into(),
                State::FundsArrival(inner) => Handler::heal(inner, querier, env, info).map_into(),
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::FundsArrival(inner) => Handler::reply(inner, querier, env, msg),
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
                State::FundsArrival(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
            }
        }

        /// Remote-lease controller callbacks reach only the stage that
        /// scheduled a remote operation - the transfer-out one. The
        /// arrival stage absorbs them with an event via the [`Handler`]
        /// defaults.
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
                State::FundsArrival(inner) => {
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
                State::FundsArrival(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_remote_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
                State::FundsArrival(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use finance::{duration::Duration, instant::Instant};
    use sdk::cosmwasm_std::QuerierWrapper;

    use crate::{Contract, impl_::remote_transfer_out::RemoteTransferOutTask};

    use super::State;

    impl<Task> Contract for State<Task>
    where
        Task: RemoteTransferOutTask,
    {
        type StateResponse = Task::StateResponse;

        fn state(
            self,
            now: Instant,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::TransferOut(inner) => Contract::state(inner, now, due_projection, querier),
                State::FundsArrival(inner) => Contract::state(inner, now, due_projection, querier),
            }
        }
    }
}

mod impl_display {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    use crate::impl_::remote_transfer_out::RemoteTransferOutTask;

    use super::State;

    impl<Task> Display for State<Task>
    where
        Task: RemoteTransferOutTask,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                State::TransferOut(inner) => Display::fmt(inner, f),
                State::FundsArrival(inner) => Display::fmt(inner, f),
            }
        }
    }
}
