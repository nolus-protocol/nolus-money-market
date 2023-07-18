use serde::{Deserialize, Serialize};

use crate::{
    SwapExactIn, SwapExactInPostRecoverIca, SwapExactInPreRecoverIca, SwapExactInRecoverIca,
    SwapExactInRecoverIcaRespDelivery, SwapExactInRespDelivery, TransferOut,
    TransferOutRespDelivery,
};

use super::swap_task::SwapTask as SwapTaskT;

#[derive(Serialize, Deserialize)]
pub enum State<SwapTask, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    TransferOut(TransferOut<SwapTask, Self>),
    TransferOutRespDelivery(TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>),
    SwapExactIn(SwapExactIn<SwapTask, Self>),
    SwapExactInRespDelivery(SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>),
    SwapExactInRecoverIcaRespDelivery(
        SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerMsg>,
    ),
    SwapExactInPreRecoverIca(SwapExactInPreRecoverIca<SwapTask, Self>),
    SwapExactInRecoverIca(SwapExactInRecoverIca<SwapTask, Self>),
    SwapExactInPostRecoverIca(SwapExactInPostRecoverIca<SwapTask, Self>),
}

pub type StartLocalRemoteState<SwapTask, ForwardToInnerMsg> =
    TransferOut<SwapTask, State<SwapTask, ForwardToInnerMsg>>;

pub fn start<SwapTask, ForwardToInnerMsg>(
    spec: SwapTask,
) -> StartLocalRemoteState<SwapTask, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    StartLocalRemoteState::new(spec)
}

mod impl_into {
    use crate::{
        swap_task::SwapTask as SwapTaskT, SwapExactIn, SwapExactInPostRecoverIca,
        SwapExactInPreRecoverIca, SwapExactInRecoverIca, SwapExactInRecoverIcaRespDelivery,
        SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery,
    };

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> From<TransferOut<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg>
        From<TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<SwapExactIn<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg>
        From<SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>) -> Self {
            Self::SwapExactInRespDelivery(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<SwapExactInPreRecoverIca<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPreRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPreRecoverIca(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<SwapExactInRecoverIca<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInRecoverIca(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg>
        From<SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerMsg>,
        ) -> Self {
            Self::SwapExactInRecoverIcaRespDelivery(value)
        }
    }

    impl<SwapTask, ForwardToInnerMsg> From<SwapExactInPostRecoverIca<SwapTask, Self>>
        for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPostRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPostRecoverIca(value)
        }
    }
}

mod impl_handler {
    use sdk::cosmwasm_std::{Binary, Deps, DepsMut, Env, Reply};

    use crate::{
        response::{ContinueResult, Result},
        swap_task::SwapTask as SwapTaskT,
        ForwardToInner, Handler,
    };

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> Handler for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        SwapTask::OutG: Clone,
        ForwardToInnerMsg: ForwardToInner,
    {
        type Response = Self;
        type SwapResult = SwapTask::Result;

        fn on_open_ica(
            self,
            counterparty_version: String,
            deps: Deps<'_>,
            env: Env,
        ) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
            }
        }

        fn on_response(self, response: Binary, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => crate::forward_to_inner(inner, response, env),

                State::TransferOutRespDelivery(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactIn(inner) => crate::forward_to_inner(inner, response, env),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
            }
        }

        fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_error(inner, deps, env),
                State::TransferOutRespDelivery(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactIn(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_error(inner, deps, env)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_error(inner, deps, env),
            }
        }

        fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactIn(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_timeout(inner, deps, env)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
            }
        }

        fn on_inner(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner(inner, deps, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_inner(inner, deps, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_inner(inner, deps, env).map_into()
                }
            }
        }

        fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::reply(inner, deps, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, deps, env, msg),
                State::SwapExactIn(inner) => Handler::reply(inner, deps, env, msg),
                State::SwapExactInRespDelivery(inner) => Handler::reply(inner, deps, env, msg),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::reply(inner, deps, env, msg)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::reply(inner, deps, env, msg),
                State::SwapExactInRecoverIca(inner) => Handler::reply(inner, deps, env, msg),
                State::SwapExactInPostRecoverIca(inner) => Handler::reply(inner, deps, env, msg),
            }
        }

        fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

    use crate::{
        swap_task::SwapTask as SwapTaskT, Contract, ContractInSwap, SwapState, TransferOutState,
    };

    use super::State;

    impl<SwapTask, ForwardToInnerMsg> Contract for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT
            + ContractInSwap<TransferOutState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<SwapState, <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = SwapTask::StateResponse;

        fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
            match self {
                State::TransferOut(inner) => Contract::state(inner, now, querier),
                State::TransferOutRespDelivery(inner) => Contract::state(inner, now, querier),
                State::SwapExactIn(inner) => Contract::state(inner, now, querier),
                State::SwapExactInRespDelivery(inner) => Contract::state(inner, now, querier),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Contract::state(inner, now, querier)
                }
                State::SwapExactInPreRecoverIca(inner) => Contract::state(inner, now, querier),
                State::SwapExactInRecoverIca(inner) => Contract::state(inner, now, querier),
                State::SwapExactInPostRecoverIca(inner) => Contract::state(inner, now, querier),
            }
        }
    }
}

mod impl_display {
    use std::fmt::Display;

    use super::State;
    use crate::swap_task::SwapTask as SwapTaskT;

    impl<SwapTask, ForwardToInnerMsg> Display for State<SwapTask, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                State::TransferOut(inner) => Display::fmt(inner, f),
                State::TransferOutRespDelivery(inner) => Display::fmt(inner, f),
                State::SwapExactIn(inner) => Display::fmt(inner, f),
                State::SwapExactInRespDelivery(inner) => Display::fmt(inner, f),
                State::SwapExactInRecoverIcaRespDelivery(inner) => Display::fmt(inner, f),
                State::SwapExactInPreRecoverIca(inner) => Display::fmt(inner, f),
                State::SwapExactInRecoverIca(inner) => Display::fmt(inner, f),
                State::SwapExactInPostRecoverIca(inner) => Display::fmt(inner, f),
            }
        }
    }
}
