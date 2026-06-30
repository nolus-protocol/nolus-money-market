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
            .all_received(self.spec.arrival_account(&env.contract.address), querier)
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

    fn heal(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        _info: &MessageInfo,
    ) -> HandlerResult<Self> {
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

#[cfg(test)]
mod tests {
    use currency::test::SuperGroupTestC1;
    use cw_time::IntoInstant;
    use finance::{
        coin::{Amount, Coin, CoinDTO},
        duration::Duration,
    };
    use platform::{batch::Emit, batch::Emitter, message::Response as MessageResponse};
    use sdk::cosmwasm_std::{
        Addr, Env, MessageInfo, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        Contract,
        impl_::{
            drain::State as DrainState,
            remote_transfer_out::mock::{self, MockSpec},
            response::{Handler, Result as HandlerResult},
            transfer_in,
        },
    };

    use super::FundsArrival;

    type Arrival = FundsArrival<MockSpec, DrainState<MockSpec>>;

    #[test]
    fn alarm_retries_while_funds_are_missing() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        let resp = match arrival(false).on_time_alarm(querier, env.clone(), alarms_delivery()) {
            HandlerResult::Continue(Ok(resp)) => resp,
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_result) => panic!("expected a continuation, got a finish"),
        };
        assert!(matches!(resp.next_state, DrainState::FundsArrival(_)));
        assert_eq!(waiting_response(&env), resp.response);
    }

    #[test]
    fn alarm_completes_when_funds_arrived() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        assert_eq!(
            mock::FINISH_RESULT,
            finished(arrival(true).on_time_alarm(querier, testing::mock_env(), alarms_delivery()))
        );
    }

    #[test]
    fn arrival_polls_the_task_supplied_account() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        // The mock reports arrival only when polled with this sub-account,
        // which differs from `env.contract.address`; completing proves the gate
        // routes the poll through `arrival_account`, not the contract address.
        let sub_account = Addr::unchecked("drain-sub-account");
        assert_ne!(sub_account, env.contract.address);
        let mut spec = MockSpec::new(vec![coin(100)]);
        spec.set_received(true);
        spec.set_arrival_account(sub_account);

        assert_eq!(
            mock::FINISH_RESULT,
            finished(Arrival::new(spec).on_time_alarm(querier, env, alarms_delivery()))
        );
    }

    #[test]
    fn alarm_from_a_stranger_rejected() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        assert!(matches!(
            arrival(true).on_time_alarm(
                querier,
                testing::mock_env(),
                MessageInfo {
                    sender: Addr::unchecked("stranger"),
                    funds: vec![],
                }
            ),
            HandlerResult::Continue(Err(_unauthorized))
        ));
    }

    #[test]
    fn heal_completes_when_funds_arrived() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        assert_eq!(
            mock::FINISH_RESULT,
            finished(arrival(true).heal(querier, testing::mock_env(), &alarms_delivery()))
        );
    }

    #[test]
    fn state_serde_round_trips() {
        let arrival = arrival(false);
        let serialized = sdk::cosmwasm_std::to_json_vec(&arrival).expect("a serializable state");
        let restored: Arrival =
            sdk::cosmwasm_std::from_json(&serialized).expect("the state should round-trip");
        assert_eq!(
            serialized,
            sdk::cosmwasm_std::to_json_vec(&restored).expect("a serializable state")
        );
    }

    #[test]
    fn contract_state_reports_the_arrival_stage() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);
        let env = testing::mock_env();

        assert_eq!(
            None,
            arrival(false).state(
                env.block.time.into_instant(),
                Duration::from_secs(0),
                querier
            )
        );
    }

    fn arrival(received: bool) -> Arrival {
        let mut spec = MockSpec::new(vec![coin(100)]);
        spec.set_received(received);
        Arrival::new(spec)
    }

    fn alarms_delivery() -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(mock::TIME_ALARMS),
            funds: vec![],
        }
    }

    fn finished(res: HandlerResult<Arrival>) -> &'static str {
        match res {
            HandlerResult::Finished(result) => result,
            HandlerResult::Continue(_resp) => panic!("expected a finish, got a continuation"),
        }
    }

    fn waiting_response(env: &Env) -> MessageResponse {
        MessageResponse::messages_with_event(
            transfer_in::setup_alarm(
                &timealarms::stub::TimeAlarmsRef::unchecked(mock::TIME_ALARMS),
                env.block.time.into_instant(),
            )
            .expect("a valid alarm setup"),
            Emitter::of_type(mock::LABEL).emit("stage", "funds-arrival"),
        )
    }

    fn coin(amount: Amount) -> CoinDTO<<MockSpec as super::RemoteTransferOutTask>::G> {
        Coin::<SuperGroupTestC1>::new(amount).into()
    }
}
