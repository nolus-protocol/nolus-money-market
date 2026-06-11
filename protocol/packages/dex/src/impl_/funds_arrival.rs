//! Local-arrival completion of a remote-account drain
//!
//! Every transfer is acknowledged, yet the funds travel on the paired
//! ICS-20 transfer channel and IBC orders nothing across channels. This
//! state polls the local account on a time-alarm cadence until every
//! transferred coin has landed, then finishes the task. There is nothing
//! to re-emit on this side - the remote account has already initiated the
//! transfers - so unlike
//! [`TransferInFinish`][super::transfer_in_finish::TransferInFinish] the
//! poll never re-enters an init stage; it only waits.

use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use finance::{duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
use timealarms::stub::TimeAlarmDelivery;

use crate::{
    Contract, TimeAlarm,
    error::{Error, Result},
    impl_::{
        remote_transfer_out::{DrainStage, RemoteTransferOutTask},
        response::{self, Handler, Result as HandlerResult},
        transfer_in,
    },
};
use cw_time::IntoInstant;

const EVENT_KEY_STAGE: &str = "stage";
const EVENT_VALUE_STAGE: &str = "funds-arrival";

/// Await the arrival of drained coins on the local account
#[derive(Serialize, Deserialize)]
#[serde(
    bound(
        serialize = "Task: Serialize",
        deserialize = "Task: Deserialize<'de> + RemoteTransferOutTask"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    spec: Task,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<Task, SEnum> FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    pub(super) fn new(spec: Task) -> Self {
        Self {
            spec,
            _state_enum: PhantomData,
        }
    }

    fn emit_waiting(&self) -> Emitter {
        Emitter::of_type(self.spec.label()).emit(EVENT_KEY_STAGE, EVENT_VALUE_STAGE)
    }
}

impl<Task, SEnum> FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
    Self: Handler<Response = SEnum, SwapResult = Task::Result> + Into<SEnum>,
{
    pub(super) fn try_complete(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.spec
            .all_received(&env.contract.address, querier)
            .map_or_else(Into::into, |received| {
                if received {
                    response::res_finished(self.spec.finish(&env, querier))
                } else {
                    self.wait(env)
                }
            })
    }

    fn wait(self, env: Env) -> HandlerResult<Self> {
        transfer_in::setup_alarm(self.spec.time_alarm(), env.block.time.into_instant())
            .and_then(|batch| {
                response::res_continue::<_, _, Self>(
                    MessageResponse::messages_with_event(batch, self.emit_waiting()),
                    self,
                )
            })
            .into()
    }
}

impl<Task, SEnum> Handler for FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
    Self: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = Task::Result;

    fn authz_remote_callback(&self, querier: QuerierWrapper<'_>, info: &MessageInfo) -> Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

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
            .map_err(Error::Unauthorized)
            .map_or_else(Into::into, |()| self.try_complete(querier, env))
    }
}

impl<Task, SEnum> Contract for FundsArrival<Task, SEnum>
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
        self.spec
            .state(DrainStage::FundsArrival, now, due_projection, querier)
    }
}

impl<Task, SEnum> Display for FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("FundsArrival at ")
            .and_then(|()| f.write_str(&self.spec.label().into()))
    }
}

impl<Task, SEnum> TimeAlarm for FundsArrival<Task, SEnum>
where
    Task: RemoteTransferOutTask,
{
    fn setup_alarm(&self, r#for: Instant) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}
