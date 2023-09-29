use serde::{Deserialize, Serialize};

use crate::{
    resp_delivery::ICAOpenResponseDelivery, DexConnectable, IcaConnectee, IcaConnector,
    SwapExactIn, SwapExactInPostRecoverIca, SwapExactInPreRecoverIca, SwapExactInRecoverIca,
    SwapExactInRecoverIcaRespDelivery, SwapExactInRespDelivery, TransferOut,
    TransferOutRespDelivery,
};

use super::swap_task::SwapTask as SwapTaskT;

pub type OpenIcaRespDelivery<OpenIca, SwapResult, ForwardToInnerMsg> =
    ICAOpenResponseDelivery<IcaConnector<OpenIca, SwapResult>, ForwardToInnerMsg>;

#[derive(Serialize, Deserialize)]
pub enum State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
where
    SwapTask: SwapTaskT,
{
    OpenIca(IcaConnector<OpenIca, SwapTask::Result>),
    OpenIcaRespDelivery(OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>),
    TransferOut(TransferOut<SwapTask, Self>),
    TransferOutRespDelivery(TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>),
    SwapExactIn(SwapExactIn<SwapTask, Self>),
    SwapExactInRespDelivery(SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>),
    SwapExactInPreRecoverIca(SwapExactInPreRecoverIca<SwapTask, Self>),
    SwapExactInRecoverIca(SwapExactInRecoverIca<SwapTask, Self>),
    SwapExactInRecoverIcaRespDelivery(
        SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>,
    ),
    SwapExactInPostRecoverIca(SwapExactInPostRecoverIca<SwapTask, Self>),
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
    use crate::{
        swap_task::SwapTask as SwapTaskT, IcaConnector, SwapExactIn, SwapExactInPostRecoverIca,
        SwapExactInPreRecoverIca, SwapExactInRecoverIca, SwapExactInRecoverIcaRespDelivery,
        SwapExactInRespDelivery, TransferOut, TransferOutRespDelivery,
    };

    use super::{OpenIcaRespDelivery, State};

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<IcaConnector<OpenIca, SwapTask::Result>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: IcaConnector<OpenIca, SwapTask::Result>) -> Self {
            Self::OpenIca(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: OpenIcaRespDelivery<OpenIca, SwapTask::Result, ForwardToInnerContinueMsg>,
        ) -> Self {
            Self::OpenIcaRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOut<SwapTask, Self>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOut<SwapTask, Self>) -> Self {
            Self::TransferOut(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: TransferOutRespDelivery<SwapTask, Self, ForwardToInnerMsg>) -> Self {
            Self::TransferOutRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactIn<SwapTask, Self>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactIn<SwapTask, Self>) -> Self {
            Self::SwapExactIn(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRespDelivery<SwapTask, Self, ForwardToInnerMsg>) -> Self {
            Self::SwapExactInRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInPreRecoverIca<SwapTask, Self>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPreRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPreRecoverIca(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRecoverIca<SwapTask, Self>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInRecoverIca(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(
            value: SwapExactInRecoverIcaRespDelivery<SwapTask, Self, ForwardToInnerContinueMsg>,
        ) -> Self {
            Self::SwapExactInRecoverIcaRespDelivery(value)
        }
    }

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        From<SwapExactInPostRecoverIca<SwapTask, Self>>
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SwapExactInPostRecoverIca<SwapTask, Self>) -> Self {
            Self::SwapExactInPostRecoverIca(value)
        }
    }
}

mod impl_handler {
    use std::fmt::Display;

    use sdk::cosmwasm_std::{Binary, Deps, DepsMut, Env, Reply};

    use crate::{
        response::{ContinueResult, Result},
        swap_task::SwapTask as SwapTaskT,
        DexConnectable, ForwardToInner, Handler, IcaConnectee, TimeAlarm,
    };

    use super::State;

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: DexConnectable + IcaConnectee<State = Self> + TimeAlarm + Display,
        SwapTask: SwapTaskT,
        SwapTask::OutG: Clone,
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

        fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::on_error(inner, deps, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_error(inner, deps, env),
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
                State::OpenIca(inner) => Handler::on_timeout(inner, deps, env),
                State::OpenIcaRespDelivery(inner) => Handler::on_timeout(inner, deps, env),
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

        fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::OpenIca(inner) => Handler::reply(inner, deps, env, msg),
                State::OpenIcaRespDelivery(inner) => Handler::reply(inner, deps, env, msg),
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

    use crate::{
        swap_task::SwapTask as SwapTaskT, Contract, ContractInSwap, SwapState, TransferOutState,
    };

    use super::State;

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg> Contract
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        OpenIca: Contract<StateResponse = SwapTask::StateResponse>,
        SwapTask: SwapTaskT
            + ContractInSwap<TransferOutState, <SwapTask as SwapTaskT>::StateResponse>
            + ContractInSwap<SwapState, <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = SwapTask::StateResponse;

        fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
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
    use crate::swap_task::SwapTask as SwapTaskT;

    impl<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg> Display
        for State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
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

    use super::{OpenIcaRespDelivery, State};
    use crate::{
        migration::MigrateSpec, swap_task::SwapTask as SwapTaskT, DexConnectable, ForwardToInner,
        IcaConnectee, IcaConnector,
    };

    //cannot impl MigrateSpec due to the need to migrate OpenIca as well
    impl<SwapTask, OpenIca, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        State<OpenIca, SwapTask, ForwardToInnerMsg, ForwardToInnerContinueMsg>
    where
        SwapTask: SwapTaskT,
        ForwardToInnerMsg: ForwardToInner,
    {
        pub fn migrate<MigrateOpenIcaFn, MigrateSpecFn, OpenIcaNew, SwapTaskNew>(
            self,
            migrate_open_ica: MigrateOpenIcaFn,
            migrate_spec: MigrateSpecFn,
        ) -> State<OpenIcaNew, SwapTaskNew, ForwardToInnerMsg, ForwardToInnerContinueMsg>
        where
            OpenIca: MigrateSpec<
                OpenIca,
                OpenIcaNew,
                State<OpenIcaNew, SwapTaskNew, ForwardToInnerMsg, ForwardToInnerContinueMsg>,
            >,
            OpenIca::Out: IcaConnectee + DexConnectable,
            IcaConnector<OpenIca::Out, SwapTask::Result>:
                Into<State<OpenIcaNew, SwapTaskNew, ForwardToInnerMsg, ForwardToInnerContinueMsg>>,
            OpenIcaRespDelivery<OpenIca::Out, SwapTask::Result, ForwardToInnerContinueMsg>:
                Into<State<OpenIcaNew, SwapTaskNew, ForwardToInnerMsg, ForwardToInnerContinueMsg>>,
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
