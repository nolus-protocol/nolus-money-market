use serde::{Deserialize, Serialize};

use crate::{
    ForwardToInner, SwapExactIn, SwapExactInRecoverIca, SwapExactInRecoverIcaRespDelivery,
    SwapExactInRespDelivery, TransferInFinish, TransferInInit, TransferInInitRecoverIca,
    TransferInInitRecoverIcaRespDelivery, TransferInInitRespDelivery, TransferOut,
    TransferOutRespDelivery,
};

use super::swap_task::SwapTask as SwapTaskT;

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de>",
))]
pub enum State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
where
    SwapTask: SwapTaskT,
{
    TransferOut(TransferOut<SwapTask, Self, SwapGroup, SwapClient>),
    TransferOutRespDelivery(
        TransferOutRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>,
    ),
    SwapExactIn(SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>),
    SwapExactInRespDelivery(
        SwapExactInRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>,
    ),
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
    TransferInInit(TransferInInit<SwapTask, Self>),
    TransferInInitRespDelivery(TransferInInitRespDelivery<SwapTask, Self, ForwardToInnerMsg>),
    TransferInInitRecoverIca(TransferInInitRecoverIca<SwapTask, Self>),
    TransferInInitRecoverIcaRespDelivery(
        TransferInInitRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>,
    ),
    TransferInFinish(TransferInFinish<SwapTask, Self>),
}

pub type StartLocalLocalState<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
> = TransferOut<
    SwapTask,
    State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>,
    SwapGroup,
    SwapClient,
>;
pub type StartRemoteLocalState<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
> = SwapExactIn<
    SwapTask,
    State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>,
    SwapGroup,
    SwapClient,
>;
pub type StartTransferInState<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
> = TransferInInit<
    SwapTask,
    State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>,
>;

pub fn start_local_local<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
>(
    spec: SwapTask,
) -> StartLocalLocalState<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
>
where
    SwapTask: SwapTaskT,
    ForwardToInnerMsg: ForwardToInner,
{
    StartLocalLocalState::new(spec)
}

pub fn start_remote_local<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
>(
    spec: SwapTask,
) -> StartRemoteLocalState<
    SwapTask,
    SwapGroup,
    SwapClient,
    ForwardToInnerMsg,
    ForwardToInnerContinueMsg,
>
where
    SwapTask: SwapTaskT,
    ForwardToInnerMsg: ForwardToInner,
{
    StartRemoteLocalState::new(spec)
}

mod impl_into {
    use crate::impl_::{
        swap_task::SwapTask as SwapTaskT, ForwardToInner, SwapExactIn, SwapExactInRecoverIca,
        SwapExactInRecoverIcaRespDelivery, TransferInFinish, TransferInInit,
        TransferInInitRecoverIca, TransferInInitRecoverIcaRespDelivery, TransferInInitRespDelivery,
        TransferOut, TransferOutRespDelivery,
    };

    use super::{State, SwapExactInRespDelivery};

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOut<SwapTask, Self, SwapGroup, SwapClient>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: TransferOut<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOutRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
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

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRespDelivery<SwapTask, Self, SwapGroup, SwapClient, ForwardToInnerMsg>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<
            SwapExactInRecoverIcaRespDelivery<
                SwapTask,
                Self,
                SwapGroup,
                SwapClient,
                ForwardToInnerContinueMsg,
            >,
        > for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRecoverIca<SwapTask, Self, SwapGroup, SwapClient>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: SwapExactInRecoverIca<SwapTask, Self, SwapGroup, SwapClient>) -> Self {
            Self::SwapExactInRecoverIca(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferInInit<SwapTask, Self>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: TransferInInit<SwapTask, Self>) -> Self {
            Self::TransferInInit(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferInInitRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: TransferInInitRespDelivery<SwapTask, Self, ForwardToInnerMsg>) -> Self {
            Self::TransferInInitRespDelivery(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferInInitRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(
            value: TransferInInitRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>,
        ) -> Self {
            Self::TransferInInitRecoverIcaRespDelivery(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferInInitRecoverIca<SwapTask, Self>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: TransferInInitRecoverIca<SwapTask, Self>) -> Self {
            Self::TransferInInitRecoverIca(value)
        }
    }

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferInFinish<SwapTask, Self>>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        fn from(value: TransferInFinish<SwapTask, Self>) -> Self {
            Self::TransferInFinish(value)
        }
    }
}

mod impl_handler {
    use currency::Group;
    use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Reply};

    use crate::{
        impl_::{
            response::{ContinueResult, Result},
            swap_task::SwapTask as SwapTaskT,
            Handler,
        },
        swap::ExactAmountIn,
    };

    use super::{ForwardToInner, State};

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
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
                State::SwapExactIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::SwapExactInRecoverIca(inner) => {
                    crate::forward_to_inner_ica::<_, ForwardToInnerContinueMsg, Self>(
                        inner,
                        counterparty_version,
                        env,
                    )
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::TransferInInit(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::TransferInInitRecoverIca(inner) => {
                    crate::forward_to_inner_ica::<_, ForwardToInnerContinueMsg, Self>(
                        inner,
                        counterparty_version,
                        env,
                    )
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::TransferInFinish(inner) => {
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
                    crate::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::SwapExactIn(inner) => {
                    crate::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::TransferInInit(inner) => {
                    crate::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }

                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::TransferInFinish(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_error(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactIn(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env)
                }
                State::SwapExactInRecoverIca(inner) => Handler::on_error(inner, querier, env),
                State::TransferInInit(inner) => Handler::on_error(inner, querier, env),
                State::TransferInInitRespDelivery(inner) => Handler::on_error(inner, querier, env),
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env)
                }
                State::TransferInInitRecoverIca(inner) => Handler::on_error(inner, querier, env),
                State::TransferInFinish(inner) => Handler::on_error(inner, querier, env),
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactIn(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_timeout(inner, querier, env)
                }
                State::SwapExactInRecoverIca(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferInInit(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_timeout(inner, querier, env)
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_timeout(inner, querier, env)
                }
                State::TransferInInitRecoverIca(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferInFinish(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::TransferInInit(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
                State::TransferInFinish(inner) => Handler::on_inner(inner, querier, env).map_into(),
            }
        }

        fn on_inner_continue(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_inner_continue(inner, querier, env),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::SwapExactIn(inner) => Handler::on_inner_continue(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::TransferInInit(inner) => Handler::on_inner_continue(inner, querier, env),
                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
                State::TransferInFinish(inner) => Handler::on_inner_continue(inner, querier, env),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::heal(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::heal(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::TransferInInit(inner) => Handler::heal(inner, querier, env).map_into(),
                State::TransferInInitRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::heal(inner, querier, env).map_into()
                }
                State::TransferInFinish(inner) => Handler::heal(inner, querier, env).map_into(),
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactIn(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::reply(inner, querier, env, msg)
                }
                State::SwapExactInRecoverIca(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferInInit(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferInInitRespDelivery(inner) => {
                    Handler::reply(inner, querier, env, msg)
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::reply(inner, querier, env, msg)
                }
                State::TransferInInitRecoverIca(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferInFinish(inner) => Handler::reply(inner, querier, env, msg),
            }
        }

        fn on_time_alarm(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_time_alarm(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_time_alarm(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferInInit(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferInInitRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIca(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferInFinish(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use finance::duration::Duration;
    use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

    use crate::impl_::{
        swap_task::SwapTask as SwapTaskT, Contract, ContractInSwap, ForwardToInner, SwapState,
        TransferInFinishState, TransferInInitState, TransferOutState,
    };

    use super::State;

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Contract
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT
            + ContractInSwap<TransferOutState, StateResponse = <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<SwapState, StateResponse = <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<
                TransferInInitState,
                StateResponse = <SwapTask as SwapTaskT>::StateResponse,
            > + ContractInSwap<
                TransferInFinishState,
                StateResponse = <SwapTask as SwapTaskT>::StateResponse,
            >,
        ForwardToInnerMsg: ForwardToInner,
    {
        type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

        fn state(
            self,
            now: Timestamp,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::TransferOut(inner) => Contract::state(inner, now, due_projection, querier),
                State::TransferOutRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::SwapExactIn(inner) => Contract::state(inner, now, due_projection, querier),
                State::SwapExactInRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::SwapExactInRecoverIca(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::TransferInInit(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::TransferInInitRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::TransferInInitRecoverIca(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
                State::TransferInFinish(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
            }
        }
    }
}

mod impl_display {
    use std::fmt::Display;

    use super::State;
    use crate::impl_::swap_task::SwapTask as SwapTaskT;

    impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Display
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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
                State::SwapExactInRecoverIca(inner) => Display::fmt(inner, f),
                State::TransferInInit(inner) => Display::fmt(inner, f),
                State::TransferInInitRespDelivery(inner) => Display::fmt(inner, f),
                State::TransferInInitRecoverIcaRespDelivery(inner) => Display::fmt(inner, f),
                State::TransferInInitRecoverIca(inner) => Display::fmt(inner, f),
                State::TransferInFinish(inner) => Display::fmt(inner, f),
            }
        }
    }
}

#[cfg(feature = "migration")]
mod impl_migration {

    use currency::Group;

    use super::State;
    use crate::{
        impl_::{
            migration::MigrateSpec, swap_task::SwapTask as SwapTaskT, ForwardToInner, InspectSpec,
        },
        swap::ExactAmountIn,
    };

    impl<
            SwapTask,
            SwapTaskNew,
            SEnumNew,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        > MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        SwapGroup: Group,
        SwapClient: ExactAmountIn,
        ForwardToInnerMsg: ForwardToInner,
        SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG, Result = SwapTask::Result>,
    {
        type Out =
            State<SwapTaskNew, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>;

        fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
        where
            MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
        {
            match self {
                State::TransferOut(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SwapExactIn(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SwapExactInRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SwapExactInRecoverIcaRespDelivery(inner) => {
                    inner.migrate_spec(migrate_fn).into()
                }
                State::SwapExactInRecoverIca(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferInInit(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferInInitRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    inner.migrate_spec(migrate_fn).into()
                }
                State::TransferInInitRecoverIca(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferInFinish(inner) => inner.migrate_spec(migrate_fn).into(),
            }
        }
    }

    impl<SwapTask, R, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        InspectSpec<SwapTask, R>
        for State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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
                State::SwapExactIn(inner) => inner.inspect_spec(inspect_fn),
                State::SwapExactInRespDelivery(inner) => inner.inspect_spec(inspect_fn),
                State::SwapExactInRecoverIcaRespDelivery(inner) => inner.inspect_spec(inspect_fn),
                State::SwapExactInRecoverIca(inner) => inner.inspect_spec(inspect_fn),
                State::TransferInInit(inner) => inner.inspect_spec(inspect_fn),
                State::TransferInInitRespDelivery(inner) => inner.inspect_spec(inspect_fn),
                State::TransferInInitRecoverIcaRespDelivery(inner) => {
                    inner.inspect_spec(inspect_fn)
                }
                State::TransferInInitRecoverIca(inner) => inner.inspect_spec(inspect_fn),
                State::TransferInFinish(inner) => inner.inspect_spec(inspect_fn),
            }
        }
    }
}
