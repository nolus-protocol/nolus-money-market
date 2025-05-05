use serde::{Deserialize, Serialize};

use crate::{
    Connectable, IcaConnectee, SwapTask as SwapTaskT,
    impl_::{
        IcaConnector, SwapExactIn, SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery,
        resp_delivery::ICAOpenResponseDelivery,
    },
};

pub type OpenIcaRespDelivery<OpenIca, SwapResult, ForwardToInnerMsg> =
    ICAOpenResponseDelivery<IcaConnector<OpenIca, SwapResult>, ForwardToInnerMsg>;

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
    TransferOut(TransferOut<SwapTask, Self, SwapClient>),
    TransferOutRespDelivery(TransferOutRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>),
    SwapExactIn(SwapExactIn<SwapTask, Self, SwapClient>),
    SwapExactInRespDelivery(SwapExactInRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>),
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
        impl_::{
            IcaConnector, SwapExactIn, SwapExactInRespDelivery, TransferOut,
            TransferOutRespDelivery,
        },
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
        From<TransferOut<SwapTask, Self, SwapClient>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self, SwapClient>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOutRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: TransferOutRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>,
        ) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactIn<SwapTask, Self, SwapClient>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self, SwapClient>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRespDelivery<SwapTask, Self, SwapClient, ForwardToInnerMsg>>
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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
    use std::fmt::Display;

    use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Reply};

    use crate::{
        Connectable, IcaConnectee, SwapTask as SwapTaskT, TimeAlarm,
        impl_::{
            self, ForwardToInner, Handler,
            response::{ContinueResult, Result},
        },
        swap::ExactAmountIn,
    };

    use super::State;

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Connectable + IcaConnectee<State = Self> + TimeAlarm + Display,
        SwapTask: SwapTaskT,
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
                State::SwapExactIn(inner) => {
                    impl_::forward_to_inner::<_, ForwardToInnerMsg, Self>(inner, response, env)
                }
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_error(inner, querier, env).map_into(),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env).map_into()
                }
                State::TransferOut(inner) => Handler::on_error(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_error(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_error(inner, querier, env).map_into()
                }
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
                State::SwapExactIn(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner(inner, querier, env).map_into()
                }
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
                State::SwapExactIn(inner) => Handler::on_inner_continue(inner, querier, env),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_inner_continue(inner, querier, env)
                }
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::heal(inner, querier, env).map_into(),
                State::OpenIcaRespDelivery(inner) => Handler::heal(inner, querier, env).map_into(),
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
                State::OpenIca(inner) => Handler::reply(inner, querier, env, msg),
                State::OpenIcaRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOut(inner) => Handler::reply(inner, querier, env, msg),
                State::TransferOutRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactIn(inner) => Handler::reply(inner, querier, env, msg),
                State::SwapExactInRespDelivery(inner) => Handler::reply(inner, querier, env, msg),
            }
        }

        fn on_time_alarm(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_time_alarm(inner, querier, env).map_into(),
                State::OpenIcaRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::TransferOut(inner) => Handler::on_time_alarm(inner, querier, env).map_into(),
                State::TransferOutRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
                State::SwapExactIn(inner) => Handler::on_time_alarm(inner, querier, env).map_into(),
                State::SwapExactInRespDelivery(inner) => {
                    Handler::on_time_alarm(inner, querier, env).map_into()
                }
            }
        }
    }
}

mod impl_contract {
    use finance::duration::Duration;
    use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

    use crate::{Contract, ContractInSwap, SwapTask as SwapTaskT};

    use super::State;

    impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Contract
        for State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Contract,
        SwapTask: SwapTaskT<StateResponse = OpenIca::StateResponse>
            + ContractInSwap<StateResponse = OpenIca::StateResponse>,
    {
        type StateResponse = OpenIca::StateResponse;

        fn state(
            self,
            now: Timestamp,
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
                State::SwapExactIn(inner) => Display::fmt(inner, f),
                State::SwapExactInRespDelivery(inner) => Display::fmt(inner, f),
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
        swap::ExactAmountIn,
    };

    //cannot impl MigrateSpec due to the need to migrate OpenIca as well
    impl<SwapTask, OpenIca, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        State<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        SwapClient: ExactAmountIn,
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
            SwapTaskNew: SwapTaskT,
        {
            match self {
                State::OpenIca(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::OpenIcaRespDelivery(inner) => inner.migrate_spec(migrate_open_ica).into(),
                State::TransferOut(inner) => inner.migrate_spec(migrate_spec).into(),
                State::TransferOutRespDelivery(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactIn(inner) => inner.migrate_spec(migrate_spec).into(),
                State::SwapExactInRespDelivery(inner) => inner.migrate_spec(migrate_spec).into(),
            }
        }
    }
}
