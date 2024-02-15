use serde::{Deserialize, Serialize};

use crate::impl_::{
    resp_delivery::ICAOpenResponseDelivery, DexConnectable, IcaConnectee, IcaConnector,
    SwapExactIn, SwapExactInPostRecoverIca, SwapExactInPreRecoverIca, SwapExactInRecoverIca,
    SwapExactInRecoverIcaRespDelivery, SwapExactInRespDelivery, TransferOut,
    TransferOutRespDelivery,
};

use super::swap_task::SwapTask as SwapTaskT;

pub type OpenIcaRespDelivery<OpenIca, SwapResult, ForwardToInnerMsg> =
    ICAOpenResponseDelivery<IcaConnector<OpenIca, SwapResult>, ForwardToInnerMsg>;

#[derive(Serialize, Deserialize)]
pub enum State<
    OpenIca,
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
> where
    SwapTask: SwapTaskT,
{
    OpenIca(IcaConnector<OpenIca, SwapTask::Result>),
    OpenIcaRespDelivery(OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>),
    TransferOut(TransferOut<SwapTask, Self, SwapGroup, SwapClient>),
    TransferOutRespDelivery(
        TransferOutRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>,
    ),
    SwapExactIn(SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>),
    SwapExactInRespDelivery(
        SwapExactInRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>,
    ),
    SwapExactInPreRecoverIca(SwapExactInPreRecoverIca<SwapTask, Self, SwapGroup, SwapClient>),
    SwapExactInRecoverIca(SwapExactInRecoverIca<SwapTask, Self, SwapGroup, SwapClient>),
    SwapExactInRecoverIcaRespDelivery(
        SwapExactInRecoverIcaRespDelivery<
            SwapTask,
            Self,
            SwapGroup,
            SwapClient,
            ForwardToInnerContinueMsg,
        >,
    ),
    SwapExactInPostRecoverIca(SwapExactInPostRecoverIca<SwapTask, Self, SwapGroup, SwapClient>),
}

pub type StartLocalRemoteState<OpenIca, SwapTask> =
    IcaConnector<OpenIca, <SwapTask as SwapTaskT>::Result>;

pub fn start<OpenIca, SwapTask>(connectee: OpenIca) -> StartLocalRemoteState<OpenIca, SwapTask>
where
    OpenIca: IcaConnectee + DexConnectable,
    SwapTask: SwapTaskT,
{
    StartLocalRemoteState::<OpenIca, SwapTask>::new(connectee)
}

mod impl_into {
    use crate::impl_::{
        swap_task::SwapTask as SwapTaskT, IcaConnector, SwapExactIn, SwapExactInPostRecoverIca,
        SwapExactInPreRecoverIca, SwapExactInRecoverIca, SwapExactInRecoverIcaRespDelivery,
        SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery,
    };

    use super::{OpenIcaRespDelivery, State};

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<IcaConnector<OpenIca, SwapTask::Result>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: IcaConnector<OpenIca, SwapTask::Result>) -> Self {
            Self::OpenIca(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>,
        ) -> Self {
            Self::OpenIcaRespDelivery(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<TransferOut<SwapTask, Self, SwapGroup, SwapClient>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
        From<TransferOutRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: TransferOutRespDelivery<
                SwapTask,
                Self,
                SwapGroup,
                SwapClient,
                ForwardToInnerMsg,
            >,
        ) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
        From<SwapExactInRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: SwapExactInRespDelivery<
                SwapTask,
                Self,
                SwapGroup,
                SwapClient,
                ForwardToInnerMsg,
            >,
        ) -> Self {
            Self::SwapExactInRespDelivery(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<SwapExactInPreRecoverIca<SwapTask, Self, SwapGroup, SwapClient>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPreRecoverIca<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactInPreRecoverIca(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<SwapExactInRecoverIca<SwapTask, Self, SwapGroup, SwapClient>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRecoverIca<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactInRecoverIca(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
        From<
            SwapExactInRecoverIcaRespDelivery<
                SwapTask,
                Self,
                SwapGroup,
                SwapClient,
                ForwardToInnerContinueMsg,
            >,
        >
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: SwapExactInRecoverIcaRespDelivery<
                SwapTask,
                Self,
                SwapGroup,
                SwapClient,
                ForwardToInnerContinueMsg,
            >,
        ) -> Self {
            Self::SwapExactInRecoverIcaRespDelivery(value)
        }
    }

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > From<SwapExactInPostRecoverIca<SwapTask, Self, SwapGroup, SwapClient>>
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPostRecoverIca<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactInPostRecoverIca(value)
        }
    }
}

mod impl_handler {
    use std::fmt::Display;

    use currency::Group;
    use sdk::cosmwasm_std::{Binary, Deps, Env, QuerierWrapper, Reply};

    use crate::{
        impl_::{
            response::{ContinueResult, Result},
            swap_task::SwapTask as SwapTaskT,
            DexConnectable, ForwardToInner, Handler, IcaConnectee, TimeAlarm,
        },
        swap::ExactAmountIn,
    };

    use super::State;

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > Handler
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        OpenIca: DexConnectable + IcaConnectee<State = Self> + TimeAlarm + Display,
        SwapTask: SwapTaskT,
        SwapTask::OutG: Clone,
        SwapGroup: Group,
        SwapClient: ExactAmountIn,
        ForwardToInnerMsg: ForwardToInner,
        ForwardToInnerContinueMsg: ForwardToInner,
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
                State::OpenIca(inner) => crate::forward_to_inner_ica::<
                    _,
                    ForwardToInnerContinueMsg,
                    Self,
                >(inner, counterparty_version, env),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
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
                State::SwapExactInRecoverIca(inner) => {
                    crate::forward_to_inner_ica::<_, ForwardToInnerContinueMsg, Self>(
                        inner,
                        counterparty_version,
                        env,
                    )
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, deps, env)
                }
            }
        }

        fn on_response(self, response: Binary, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::TransferOut(inner) => {
                    crate::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }

                State::TransferOutRespDelivery(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_response(inner, response, deps, env).map_into()
                }
                State::SwapExactIn(inner) => {
                    crate::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
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

        fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_error(inner, querier, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::TransferOut(inner) => Handler::on_error(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactIn(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_error(inner, querier, env),
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_timeout(inner, querier, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactIn(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_timeout(inner, querier, env)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRecoverIca(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInPostRecoverIca(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_inner(inner, deps, env).map_into(),
                State::OpenIcaRespDelivery(inner) => Handler::on_inner(inner, deps, env).map_into(),
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

        fn on_inner_continue(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_inner_continue(inner, deps, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_inner_continue(inner, deps, env),
                State::TransferOut(inner) => Handler::on_inner_continue(inner, deps, env),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, deps, env)
                }
                State::SwapExactIn(inner) => Handler::on_inner_continue(inner, deps, env),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, deps, env)
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, deps, env)
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::on_inner_continue(inner, deps, env)
                }
                State::SwapExactInRecoverIca(inner) => Handler::on_inner_continue(inner, deps, env),
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::on_inner_continue(inner, deps, env)
                }
            }
        }

        fn heal(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::heal(inner, deps, env).map_into(),
                State::OpenIcaRespDelivery(inner) => Handler::heal(inner, deps, env).map_into(),
                State::TransferOut(inner) => Handler::heal(inner, deps, env).map_into(),
                State::TransferOutRespDelivery(inner) => Handler::heal(inner, deps, env).map_into(),
                State::SwapExactIn(inner) => Handler::heal(inner, deps, env).map_into(),
                State::SwapExactInRespDelivery(inner) => Handler::heal(inner, deps, env).map_into(),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::heal(inner, deps, env).map_into()
                }
                State::SwapExactInPreRecoverIca(inner) => {
                    Handler::heal(inner, deps, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => Handler::heal(inner, deps, env).map_into(),
                State::SwapExactInPostRecoverIca(inner) => {
                    Handler::heal(inner, deps, env).map_into()
                }
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::reply(inner, querier, env, msg),
                State::OpenIcaRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactIn(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::reply(inner, querier, env, msg)
                }
                State::SwapExactInPreRecoverIca(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRecoverIca(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInPostRecoverIca(inner) => Handler::reply(inner, querier, env, msg),
            }
        }

        fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_time_alarm(inner, deps, env).map_into(),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, deps, env).map_into()
                }
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

    use crate::impl_::{
        swap_task::SwapTask as SwapTaskT, Contract, ContractInSwap, SwapState, TransferOutState,
    };

    use super::State;

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > Contract
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        OpenIca: Contract<StateResponse = SwapTask::StateResponse>,
        SwapTask: SwapTaskT
            + ContractInSwap<TransferOutState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<SwapState, <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = SwapTask::StateResponse;

        fn state(self, now: Timestamp, querier: QuerierWrapper<'_>) -> Self::StateResponse {
            match self {
                State::OpenIca(inner) => Contract::state(inner, now, querier),
                State::OpenIcaRespDelivery(inner) => Contract::state(inner, now, querier),
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
    use crate::impl_::swap_task::SwapTask as SwapTaskT;

    impl<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > Display
        for State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
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

#[cfg(feature = "migration")]
mod impl_migration {

    use currency::Group;

    use super::{OpenIcaRespDelivery, State};
    use crate::{
        impl_::{
            migration::MigrateSpec, swap_task::SwapTask as SwapTaskT, DexConnectable,
            ForwardToInner, IcaConnectee, IcaConnector,
        },
        swap::ExactAmountIn,
    };

    //cannot impl MigrateSpec due to the need to migrate OpenIca as well
    impl<
            SwapTask,
            OpenIca,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
        State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
    where
        SwapTask: SwapTaskT,
        SwapGroup: Group,
        SwapClient: ExactAmountIn,
        ForwardToInnerMsg: ForwardToInner,
    {
        pub fn migrate<MigrateOpenIcaFn, MigrateSpecFn, OpenIcaNew, SwapTaskNew>(
            self,
            migrate_open_ica: MigrateOpenIcaFn,
            migrate_spec: MigrateSpecFn,
        ) -> State<
            OpenIcaNew,
            SwapTaskNew,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >
        where
            OpenIca: MigrateSpec<
                OpenIca,
                OpenIcaNew,
                State<
                    OpenIcaNew,
                    SwapTaskNew,
                    SwapGroup,
                    SwapClient,
                    ForwardToInnerMsg,
                    ForwardToInnerContinueMsg,
                >,
            >,
            OpenIca::Out: IcaConnectee + DexConnectable,
            IcaConnector<OpenIca::Out, SwapTask::Result>: Into<
                State<
                    OpenIcaNew,
                    SwapTaskNew,
                    SwapGroup,
                    SwapClient,
                    ForwardToInnerMsg,
                    ForwardToInnerContinueMsg,
                >,
            >,
            OpenIcaRespDelivery<OpenIca::Out, SwapTask::Result, ForwardToInnerContinueMsg>: Into<
                State<
                    OpenIcaNew,
                    SwapTaskNew,
                    SwapGroup,
                    SwapClient,
                    ForwardToInnerMsg,
                    ForwardToInnerContinueMsg,
                >,
            >,
            MigrateOpenIcaFn: FnOnce(OpenIca) -> OpenIcaNew,
            MigrateSpecFn: FnOnce(SwapTask) -> SwapTaskNew,
            SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG, Result = SwapTask::Result>,
        {
            match self {
                State::OpenIca(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::OpenIcaRespDelivery(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::TransferOut(inner) => inner.migrate_spec(migrate_spec).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactIn(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactInRespDelivery(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    inner.migrate_spec(migrate_spec).into()
                }
                State::SwapExactInPreRecoverIca(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactInRecoverIca(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactInPostRecoverIca(inner) => inner.migrate_spec(migrate_spec).into(),
            }
        }
    }
}
