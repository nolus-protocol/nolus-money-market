use std::fmt::{Display, Formatter, Result as FmtResult};
use std::marker::PhantomData;

use crate::Error as DexError;
use currency::{CurrencyDef, MemberOf};
use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use finance::coin::CoinDTO;
use platform::{
    batch::{Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmDelivery;

use crate::{
    Contract, ContractInSwap, Enterable, Stage, SwapOutputTask, SwapTask as SwapTaskT,
    WithOutputTask,
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};
use super::{
    response::{self, Handler as HandlerT, Result as HandlerResult},
    transfer_in,
    transfer_in_init::TransferInInit,
};

#[derive(Serialize, Deserialize)]
pub struct TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    amount_in: CoinDTO<SwapTask::OutG>,
    timeout: Timestamp,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<SwapTask, SEnum> TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub(super) fn new(
        spec: SwapTask,
        amount_in: CoinDTO<SwapTask::OutG>,
        timeout: Timestamp,
    ) -> Self {
        Self {
            spec,
            amount_in,
            timeout,
            _state_enum: Default::default(),
        }
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SEnum> TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub fn into_init(self) -> TransferInInit<SwapTask, SEnum> {
        TransferInInit::new(self.spec, self.amount_in)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew> MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    SwapTaskNew: SwapTaskT<OutG = SwapTask::OutG>,
{
    type Out = TransferInFinish<SwapTaskNew, SEnumNew>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec), self.amount_in, self.timeout)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum> InspectSpec<SwapTask, R> for TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

impl<SwapTask, SEnum> TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    TransferInInit<SwapTask, SEnum>: Into<SEnum>,
{
    pub(super) fn try_complete(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        struct FinishOrTryAgainCmd<'querier, SwapTask, Handler>
        where
            SwapTask: SwapTaskT,
        {
            amount_in: CoinDTO<SwapTask::OutG>,
            timeout: Timestamp,
            querier: QuerierWrapper<'querier>,
            env: Env,
            _task: PhantomData<SwapTask>,
            _handler: PhantomData<Handler>,
        }

        impl<'querier, SwapTask, Handler> FinishOrTryAgainCmd<'querier, SwapTask, Handler>
        where
            SwapTask: SwapTaskT,
        {
            fn from(
                amount_in: CoinDTO<SwapTask::OutG>,
                timeout: Timestamp,
                querier: QuerierWrapper<'querier>,
                env: Env,
            ) -> Self {
                Self {
                    amount_in,
                    timeout,
                    querier,
                    env,
                    _task: PhantomData,
                    _handler: PhantomData,
                }
            }
        }

        impl<SwapTask, Handler> FinishOrTryAgainCmd<'_, SwapTask, Handler>
        where
            SwapTask: SwapTaskT,
            Handler: HandlerT<SwapResult = SwapTask::Result>,
            TransferInInit<SwapTask, Handler::Response>: Into<Handler::Response>,
            TransferInFinish<SwapTask, Handler::Response>: Into<Handler::Response>,
        {
            fn try_again(self, spec: SwapTask) -> HandlerResult<Handler> {
                let now = self.env.block.time;
                let emitter = self.emit_ok(&spec);
                if now >= self.timeout {
                    let next_state = TransferInInit::new(spec, self.amount_in);
                    next_state
                        .enter(now, self.querier)
                        .map(|batch| MessageResponse::messages_with_events(batch, emitter))
                        .and_then(|resp| response::res_continue::<_, _, Handler>(resp, next_state))
                        .into()
                } else {
                    transfer_in::setup_alarm(spec.time_alarm(), now)
                        .map(|batch| MessageResponse::messages_with_events(batch, emitter))
                        .and_then(|resp| {
                            response::res_continue::<_, _, Handler>(resp, self.back_to_spec(spec))
                        })
                        .into()
                }
            }

            fn emit_ok(&self, spec: &SwapTask) -> Emitter {
                Emitter::of_type(spec.label())
                    .emit("stage", "transfer-in")
                    .emit_coin_dto("amount", &self.amount_in)
            }

            fn back_to_spec(self, spec: SwapTask) -> TransferInFinish<SwapTask, Handler::Response> {
                TransferInFinish::new(spec, self.amount_in, self.timeout)
            }
        }

        impl<SwapTask, Handler> WithOutputTask<SwapTask> for FinishOrTryAgainCmd<'_, SwapTask, Handler>
        where
            SwapTask: SwapTaskT,
            Handler: HandlerT<SwapResult = SwapTask::Result>,
            TransferInInit<SwapTask, Handler::Response>: Into<Handler::Response>,
            TransferInFinish<SwapTask, Handler::Response>: Into<Handler::Response>,
        {
            type Output = HandlerResult<Handler>;

            fn on<OutC, OutputTaskT>(self, task: OutputTaskT) -> Self::Output
            where
                OutC: CurrencyDef,
                OutC::Group: MemberOf<SwapTask::OutG>,
                OutputTaskT: SwapOutputTask<SwapTask, OutC = OutC>,
            {
                let expected_amount = self
                    .amount_in
                    .as_specific(&currency::dto::<OutC, SwapTask::OutG>());

                transfer_in::check_received(
                    &expected_amount,
                    &self.env.contract.address,
                    self.querier,
                )
                .map_or_else(Into::into, |received| {
                    if received {
                        response::res_finished(task.finish(
                            expected_amount,
                            &self.env,
                            self.querier,
                        ))
                    } else {
                        self.try_again(task.into_spec())
                    }
                })
            }
        }

        self.spec.into_output_task(FinishOrTryAgainCmd::from(
            self.amount_in,
            self.timeout,
            querier,
            env,
        ))
    }
}

impl<SwapTask, SEnum> HandlerT for TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
    Self: Into<SEnum>,
    TransferInInit<SwapTask, SEnum>: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.try_complete(querier, env)
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> HandlerResult<Self> {
        access_control::check(&TimeAlarmDelivery::new(self.spec.time_alarm()), &info)
            .map_err(DexError::Unauthorized)
            .map_or_else(Into::into, |()| self.try_complete(querier, env))
    }
}

impl<SwapTask, SEnum> Contract for TransferInFinish<SwapTask, SEnum>
where
    SwapTask: SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec
            .state(Stage::TransferInFinish, now, due_projection, querier)
    }
}

impl<SwapTask, ForwardToInnerMsg> Display for TransferInFinish<SwapTask, ForwardToInnerMsg>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!(
            "TransferInFinish at {}",
            self.spec.label().into()
        ))
    }
}
