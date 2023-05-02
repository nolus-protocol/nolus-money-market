use serde::{Deserialize, Serialize};

use crate::{
    SwapExactIn, SwapExactInPostRecoverIca, SwapExactInPreRecoverIca, SwapExactInRecoverIca,
    TransferInFinish, TransferInInit, TransferInInitPostRecoverIca, TransferInInitPreRecoverIca,
    TransferInInitRecoverIca, TransferOut,
};

use super::swap_task::SwapTask as SwapTaskT;

#[derive(Serialize, Deserialize)]
pub enum State<SwapTask>
where
    SwapTask: SwapTaskT,
{
    TransferOut(TransferOut<SwapTask, Self>),
    SwapExactIn(SwapExactIn<SwapTask, Self>),
    SwapExactInPreRecoverIca(SwapExactInPreRecoverIca<SwapTask, Self>),
    SwapExactInRecoverIca(SwapExactInRecoverIca<SwapTask, Self>),
    SwapExactInPostRecoverIca(SwapExactInPostRecoverIca<SwapTask, Self>),
    TransferInInit(TransferInInit<SwapTask>),
    TransferInInitPreRecoverIca(TransferInInitPreRecoverIca<SwapTask, Self>),
    TransferInInitRecoverIca(TransferInInitRecoverIca<SwapTask, Self>),
    TransferInInitPostRecoverIca(TransferInInitPostRecoverIca<SwapTask>),
    TransferInFinish(TransferInFinish<SwapTask>),
}

pub type StartLocalLocalState<SwapTask> = TransferOut<SwapTask, State<SwapTask>>;
pub type StartRemoteLocalState<SwapTask> = SwapExactIn<SwapTask, State<SwapTask>>;

pub fn start_local_local<SwapTask>(spec: SwapTask) -> StartLocalLocalState<SwapTask>
where
    SwapTask: SwapTaskT,
{
    StartLocalLocalState::new(spec)
}

pub fn start_remote_local<SwapTask>(spec: SwapTask) -> StartRemoteLocalState<SwapTask>
where
    SwapTask: SwapTaskT,
{
    StartRemoteLocalState::new(spec)
}

mod impl_into {
    use crate::{
        swap_task::SwapTask as SwapTaskT, SwapExactIn, SwapExactInPostRecoverIca,
        SwapExactInPreRecoverIca, SwapExactInRecoverIca, TransferInFinish, TransferInInit,
        TransferInInitPostRecoverIca, TransferInInitPreRecoverIca, TransferInInitRecoverIca,
        TransferOut,
    };

    use super::State;

    impl<SwapTask> From<TransferOut<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<SwapTask> From<SwapExactIn<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<SwapTask> From<SwapExactInPreRecoverIca<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPreRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPreRecoverIca(value)
        }
    }

    impl<SwapTask> From<SwapExactInRecoverIca<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInRecoverIca(value)
        }
    }

    impl<SwapTask> From<SwapExactInPostRecoverIca<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPostRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPostRecoverIca(value)
        }
    }

    impl<SwapTask> From<TransferInInit<SwapTask>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferInInit<SwapTask>) -> Self {
            Self::TransferInInit(value)
        }
    }

    impl<SwapTask> From<TransferInInitPreRecoverIca<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferInInitPreRecoverIca<SwapTask, Self>) -> Self {
            Self::TransferInInitPreRecoverIca(value)
        }
    }

    impl<SwapTask> From<TransferInInitRecoverIca<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferInInitRecoverIca<SwapTask, Self>) -> Self {
            Self::TransferInInitRecoverIca(value)
        }
    }

    impl<SwapTask> From<TransferInInitPostRecoverIca<SwapTask>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferInInitPostRecoverIca<SwapTask>) -> Self {
            Self::TransferInInitPostRecoverIca(value)
        }
    }

    impl<SwapTask> From<TransferInFinish<SwapTask>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferInFinish<SwapTask>) -> Self {
            Self::TransferInFinish(value)
        }
    }
}

mod impl_handler {
    use sdk::cosmwasm_std::{Binary, Deps, Env};

    use crate::{
        response::{ContinueResult, Result},
        swap_task::SwapTask as SwapTaskT,
        Handler,
    };

    use super::State;

    impl<SwapTask> Handler for State<SwapTask>
    where
        SwapTask: SwapTaskT,
        SwapTask::OutG: Clone,
    {
        type Response = Self;
        type SwapResult = SwapTask::Result;

        fn on_open_ica(
            self,
            counterparty_version: String,
            deps: sdk::cosmwasm_std::Deps<'_>,
            env: sdk::cosmwasm_std::Env,
        ) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferInInit(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferInInitPreRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferInInitPostRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::TransferInFinish(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
            }
        }

        fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::SwapExactIn(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::TransferInInit(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::TransferInInitPreRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::TransferInInitPostRecoverIca(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
                State::TransferInFinish(inner) => {
                    Handler::on_response(inner, data, deps, env).map_into()
                }
            }
        }

        fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactIn(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInPreRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::TransferInInit(inner) => Handler::on_error(inner, deps, env),
                State::TransferInInitPreRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::TransferInInitRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::TransferInInitPostRecoverIca(inner) => Handler::on_error(inner, deps, env),
                State::TransferInFinish(inner) => Handler::on_error(inner, deps, env),
            }
        }

        fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactIn(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInPreRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferInInit(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferInInitPreRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferInInitRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferInInitPostRecoverIca(inner) => Handler::on_timeout(inner, deps, env),
                State::TransferInFinish(inner) => Handler::on_timeout(inner, deps, env),
            }
        }

        fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::SwapExactIn(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::TransferInInit(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::TransferInInitPreRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::TransferInInitPostRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
                State::TransferInFinish(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

    use crate::{
        swap_task::SwapTask as SwapTaskT, Contract, ContractInSwap, SwapState,
        TransferInFinishState, TransferInInitState, TransferOutState,
    };

    use super::State;

    impl<SwapTask> Contract for State<SwapTask>
    where
        SwapTask: SwapTaskT
            + ContractInSwap<TransferOutState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<SwapState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<TransferInInitState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<TransferInFinishState, <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = SwapTask::StateResponse;

        fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
            match self {
                State::TransferOut(inner) => Contract::state(inner, now, querier),
                State::SwapExactIn(inner) => Contract::state(inner, now, querier),
                State::SwapExactInPreRecoverIca(inner) => Contract::state(inner, now, querier),
                State::SwapExactInRecoverIca(inner) => Contract::state(inner, now, querier),
                State::SwapExactInPostRecoverIca(inner) => Contract::state(inner, now, querier),
                State::TransferInInit(inner) => Contract::state(inner, now, querier),
                State::TransferInInitPreRecoverIca(inner) => Contract::state(inner, now, querier),
                State::TransferInInitRecoverIca(inner) => Contract::state(inner, now, querier),
                State::TransferInInitPostRecoverIca(inner) => Contract::state(inner, now, querier),
                State::TransferInFinish(inner) => Contract::state(inner, now, querier),
            }
        }
    }
}
