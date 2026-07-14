use serde::{Deserialize, Serialize};

use crate::{
    ForwardToInner, SwapTask as SwapTaskT, TransportOutFactory as TransportOutFactoryT,
    impl_::{SwapExactIn, SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery},
};

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize,
                    TransportOutFactory: Serialize",
    deserialize = "SwapTask: Deserialize<'de>,
                    TransportOutFactory: Deserialize<'de>",
))]
pub enum State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    TransferOut(TransferOut<SwapTask, Self, TransportOutFactory, SwapClient>),
    TransferOutRespDelivery(
        TransferOutRespDelivery<SwapTask, Self, TransportOutFactory, SwapClient, ForwardToInnerMsg>,
    ),
    SwapExactIn(SwapExactIn<SwapTask, Self, SwapClient>),
    SwapExactInRespDelivery(SwapExactInRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>),
}

pub type StartLocalRemoteState<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg> =
    TransferOut<
        SwapTask,
        State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>,
        TransportOutFactory,
        SwapClient,
    >;

pub fn start<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>(
    spec: SwapTask,
    transport: TransportOutFactory,
) -> StartLocalRemoteState<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    ForwardToInnerMsg: ForwardToInner,
{
    StartLocalRemoteState::new(spec, transport)
}

mod impl_into {
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{SwapExactIn, SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery},
    };

    use super::State;

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
        From<
            TransferOut<
                SwapTask,
                State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>,
                TransportOutFactory,
                SwapClient,
            >,
        > for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self, TransportOutFactory, SwapClient>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
        From<
            TransferOutRespDelivery<
                SwapTask,
                State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>,
                TransportOutFactory,
                SwapClient,
                ForwardToInnerMsg,
            >,
        > for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: TransferOutRespDelivery<
                SwapTask,
                Self,
                TransportOutFactory,
                SwapClient,
                ForwardToInnerMsg,
            >,
        ) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
        From<SwapExactIn<SwapTask, Self, SwapClient>>
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self, SwapClient>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
        From<SwapExactInRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>>
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: SwapExactInRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>,
        ) -> Self {
            Self::SwapExactInRespDelivery(value)
        }
    }
}

mod impl_handler {
    use platform::ica::ErrorResponse as ICAErrorResponse;
    use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

    use crate::{
        SwapTask as SwapTaskT, TransportOutFactory as TransportOutFactoryT,
        impl_::{
            self, ForwardToInner, Handler,
            response::{ContinueResult, Result},
        },
        swap::ExactAmountIn,
    };

    use super::State;

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg> Handler
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        TransportOutFactory: TransportOutFactoryT,
        SwapClient: ExactAmountIn,
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
                State::SwapExactIn(inner) => inner.authz_remote_callback(querier, info),
                State::SwapExactInRespDelivery(inner) => inner.authz_remote_callback(querier, info),
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
                State::SwapExactIn(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::SwapExactInRespDelivery(inner) => {
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
                State::SwapExactIn(inner) => {
                    impl_::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::SwapExactInRespDelivery(inner) => {
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
                State::SwapExactIn(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::on_timeout(inner, querier, env),
                State::TransferOutRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactIn(inner) => Handler::on_timeout(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => Handler::on_timeout(inner, querier, env),
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
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactIn(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
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
                State::SwapExactIn(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use finance::duration::Duration;
    use finance::instant::Instant;
    use sdk::cosmwasm_std::QuerierWrapper;

    use crate::{Contract, ContractInSwap, SwapTask as SwapTaskT};

    use super::State;

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg> Contract
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask:
            SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
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
                State::SwapExactIn(inner) => Contract::state(inner, now, due_projection, querier),
                State::SwapExactInRespDelivery(inner) => {
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

    impl<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg> Display
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                State::TransferOut(inner) => Display::fmt(inner, f),
                State::TransferOutRespDelivery(inner) => Display::fmt(inner, f),
                State::SwapExactIn(inner) => Display::fmt(inner, f),
                State::SwapExactInRespDelivery(inner) => Display::fmt(inner, f),
            }
        }
    }
}

#[cfg(feature = "migration")]
mod impl_migration {
    use super::State;
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{ForwardToInner, migration::MigrateSpec},
        swap::ExactAmountIn,
    };

    impl<SwapTask, SwapTaskNew, SEnumNew, TransportOutFactory, SwapClient, ForwardToInnerMsg>
        MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
        for State<SwapTask, TransportOutFactory, SwapClient, ForwardToInnerMsg>
    where
        SwapTask: SwapTaskT,
        SwapClient: ExactAmountIn,
        ForwardToInnerMsg: ForwardToInner,
        SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
    {
        type Out = State<SwapTaskNew, TransportOutFactory, SwapClient, ForwardToInnerMsg>;

        fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
        where
            MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
        {
            match self {
                State::TransferOut(inner) => inner.migrate_spec(migrate_fn).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SwapExactIn(inner) => inner.migrate_spec(migrate_fn).into(),
                State::SwapExactInRespDelivery(inner) => inner.migrate_spec(migrate_fn).into(),
            }
        }
    }
}
