//! A swap-only composite over the remote-lease controller transport
//!
//! The asset-to-output swap legs run over the remote-lease controller, one
//! in-flight leg at a time. There is neither an ICA leg nor a transfer leg in
//! this composite; its terminal `finish` builds and enters a separate drain
//! state.

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    SwapTask as SwapTaskT,
    impl_::{RemoteSwap, RemoteSwapClient, SlippageAnomaly},
    response::Result as HandlerResult,
};

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
))]
pub enum State<SwapTask>
where
    SwapTask: SwapTaskT,
{
    RemoteSwap(RemoteSwap<SwapTask, Self>),
    SlippageAnomaly(SlippageAnomaly<SwapTask, Self>),
}

pub type StartSwapState<SwapTask> = RemoteSwap<SwapTask, State<SwapTask>>;

/// Build the workflow's entry state over the swap specification
///
/// Folds the coins already in the output currency and schedules the first
/// swap leg. A task with nothing to swap finishes synchronously.
pub fn start<SwapTask>(
    spec: SwapTask,
    env: &Env,
    querier: QuerierWrapper<'_>,
) -> HandlerResult<StartSwapState<SwapTask>>
where
    SwapTask: SwapTaskT + RemoteSwapClient,
{
    RemoteSwap::start(spec, env, querier)
}

mod impl_into {
    use crate::{
        SwapTask as SwapTaskT,
        impl_::{RemoteSwap, SlippageAnomaly},
    };

    use super::State;

    impl<SwapTask> From<RemoteSwap<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: RemoteSwap<SwapTask, Self>) -> Self {
            Self::RemoteSwap(value)
        }
    }

    impl<SwapTask> From<SlippageAnomaly<SwapTask, Self>> for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn from(value: SlippageAnomaly<SwapTask, Self>) -> Self {
            Self::SlippageAnomaly(value)
        }
    }
}

mod impl_handler {
    use platform::{batch::Emitter, ica::ErrorResponse as ICAErrorResponse};
    use sdk::cosmwasm_std::{Binary, Env, MessageInfo, QuerierWrapper, Reply};

    use crate::{
        SwapTask as SwapTaskT,
        error::Result as DexResult,
        impl_::RemoteSwapClient,
        response::{ContinueResult, Handler, Result},
    };

    use super::State;

    impl<SwapTask> Handler for State<SwapTask>
    where
        SwapTask: SwapTaskT + RemoteSwapClient,
    {
        type Response = Self;
        type SwapResult = SwapTask::Result;

        fn authz_remote_callback(
            &self,
            querier: QuerierWrapper<'_>,
            info: &MessageInfo,
        ) -> DexResult<()> {
            match self {
                State::RemoteSwap(inner) => inner.authz_remote_callback(querier, info),
                State::SlippageAnomaly(inner) => inner.authz_remote_callback(querier, info),
            }
        }

        fn on_open_ica(
            self,
            counterparty_version: String,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> ContinueResult<Self> {
            match self {
                State::RemoteSwap(inner) => {
                    Handler::on_open_ica(inner, counterparty_version, querier, env)
                }
                State::SlippageAnomaly(inner) => {
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
                State::RemoteSwap(inner) => {
                    Handler::on_response(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
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
                State::RemoteSwap(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RemoteSwap(inner) => Handler::on_timeout(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_timeout(inner, querier, env),
            }
        }

        fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => Handler::on_inner(inner, querier, env).map_into(),
                State::SlippageAnomaly(inner) => Handler::on_inner(inner, querier, env).map_into(),
            }
        }

        fn on_inner_continue(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
            match self {
                State::RemoteSwap(inner) => Handler::on_inner_continue(inner, querier, env),
                State::SlippageAnomaly(inner) => Handler::on_inner_continue(inner, querier, env),
            }
        }

        fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => Handler::heal(inner, querier, env).map_into(),
                State::SlippageAnomaly(inner) => Handler::heal(inner, querier, env).map_into(),
            }
        }

        fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: Reply) -> ContinueResult<Self> {
            match self {
                State::RemoteSwap(inner) => Handler::reply(inner, querier, env, msg),
                State::SlippageAnomaly(inner) => Handler::reply(inner, querier, env, msg),
            }
        }

        fn on_time_alarm(
            self,
            querier: QuerierWrapper<'_>,
            env: Env,
            info: MessageInfo,
        ) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_time_alarm(inner, querier, env, info).map_into()
                }
            }
        }

        fn on_remote_response(
            self,
            data: Binary,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_response(inner, data, querier, env).map_into()
                }
            }
        }

        fn on_remote_error(
            self,
            response: ICAErrorResponse,
            querier: QuerierWrapper<'_>,
            env: Env,
        ) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_error(inner, response, querier, env).map_into()
                }
            }
        }

        fn on_remote_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> Result<Self> {
            match self {
                State::RemoteSwap(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
                State::SlippageAnomaly(inner) => {
                    Handler::on_remote_timeout(inner, querier, env).map_into()
                }
            }
        }

        fn price_alarm_dropped(&self) -> Option<Emitter> {
            match self {
                State::RemoteSwap(inner) => inner.price_alarm_dropped(),
                State::SlippageAnomaly(inner) => inner.price_alarm_dropped(),
            }
        }
    }
}

mod impl_contract {
    use finance::{duration::Duration, instant::Instant};
    use sdk::cosmwasm_std::QuerierWrapper;

    use crate::{Contract, ContractInRemoteSwap, SwapTask as SwapTaskT};

    use super::State;

    impl<SwapTask> Contract for State<SwapTask>
    where
        SwapTask: SwapTaskT
            + ContractInRemoteSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
    {
        type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

        fn state(
            self,
            now: Instant,
            due_projection: Duration,
            querier: QuerierWrapper<'_>,
        ) -> Self::StateResponse {
            match self {
                State::RemoteSwap(inner) => Contract::state(inner, now, due_projection, querier),
                State::SlippageAnomaly(inner) => {
                    Contract::state(inner, now, due_projection, querier)
                }
            }
        }
    }
}

mod impl_display {
    use std::fmt::{Display, Formatter, Result as FmtResult};

    use crate::SwapTask as SwapTaskT;

    use super::State;

    impl<SwapTask> Display for State<SwapTask>
    where
        SwapTask: SwapTaskT,
    {
        fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
            match self {
                State::RemoteSwap(inner) => inner.fmt(f),
                State::SlippageAnomaly(inner) => inner.fmt(f),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use currency::test::{SuperGroupTestC1, SuperGroupTestC2};
    use finance::coin::{Amount, Coin, CoinDTO};
    use platform::{
        batch::{Emit, Emitter},
        message::Response as MessageResponse,
    };
    use sdk::cosmwasm_std::{
        Binary, QuerierWrapper,
        testing::{self, MockQuerier},
    };

    use crate::{
        impl_::remote_swap::mock::{self, MockSpec},
        response::{Handler, Result as HandlerResult},
    };

    use super::{State, start};

    type OutG = <MockSpec as crate::SwapTask>::OutG;

    const ANOMALY_FLOOR: Amount = 40;

    /// `start` folds the out-currency coins and schedules the first swap leg
    /// through the single composite arm.
    #[test]
    fn start_schedules_the_first_swap_leg() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, _state) =
            continued(start(spec3(), &testing::mock_env(), querier).map_into());
        assert_eq!(
            leg_response(&coin_in(100), &min_out(), &coin_out(50)),
            response
        );
    }

    /// The acknowledgment of the last leg drives the composite to
    /// `Result::Finished` with the accumulated total - the seam a later
    /// commit re-targets to build-and-enter the drain state.
    #[test]
    fn final_ack_drives_finish() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let state = after_first_ack(querier);
        assert_eq!(
            coin_out(120),
            finished(state.on_remote_response(
                payload(&coin_out(40)),
                querier,
                testing::mock_env()
            ))
        );
    }

    /// `heal` re-emits only the in-flight leg with its pinned floor instead
    /// of erroring, restarting the sequence, or finishing.
    #[test]
    fn heal_reemits_the_in_flight_leg() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, _state) =
            continued(after_first_ack(querier).heal(querier, testing::mock_env()));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(70), &min_out()).expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("heal", "re-emit"),
            ),
            response
        );
    }

    /// The `RemoteSwap` arm survives a serde round-trip and keeps driving
    /// the workflow afterwards - the restored arm finishes with the same
    /// accumulated total as the original would have.
    #[test]
    fn swap_arm_serde_round_trips() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let state = after_first_ack(querier);
        let serialized = sdk::cosmwasm_std::to_json_vec(&state).expect("a serializable arm");
        let restored: State<MockSpec> =
            sdk::cosmwasm_std::from_json(&serialized).expect("the swap arm should round-trip");
        assert_eq!(
            serialized,
            sdk::cosmwasm_std::to_json_vec(&restored).expect("a serializable arm"),
        );
        assert_eq!(
            coin_out(120),
            finished(restored.on_remote_response(
                payload(&coin_out(40)),
                querier,
                testing::mock_env(),
            ))
        );
    }

    /// An acknowledgment below the pinned floor re-emits the in-flight leg
    /// (a continuation), not an absorb-to-exit - liquidation loses its
    /// immediate slippage-anomaly valve on this transport.
    #[test]
    fn underpaid_ack_reemits_instead_of_exiting() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let mut spec = spec3();
        spec.set_floor(ANOMALY_FLOOR);
        let (_response, state) = continued(start(spec, &testing::mock_env(), querier).map_into());

        let (response, _state) = continued(state.on_remote_response(
            payload(&coin_out(ANOMALY_FLOOR - 1)),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(
            MessageResponse::messages_with_event(
                mock::swap_request(&coin_in(100), &coin_out(ANOMALY_FLOOR))
                    .expect("a valid swap request"),
                Emitter::of_type(mock::LABEL).emit("anomaly", "under-min-out"),
            ),
            response
        );
    }

    /// An `OperationErr` parks the composite at the `SlippageAnomaly` arm
    /// without retrying, emitting the on-entry anomaly event.
    #[test]
    fn error_parks_at_slippage_anomaly_arm() {
        use platform::ica::ErrorResponse as ICAErrorResponse;

        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, state) = continued(after_first_ack(querier).on_remote_error(
            ICAErrorResponse::from(String::from("swap reverted")),
            querier,
            testing::mock_env(),
        ));
        assert_eq!(
            MessageResponse::messages_with_event(
                Default::default(),
                Emitter::of_type(mock::LABEL).emit("anomaly", "slippage-anomaly-parked"),
            ),
            response
        );
        assert!(matches!(state, State::SlippageAnomaly(_)));
    }

    /// The parked `SlippageAnomaly` arm survives a serde round-trip and keeps
    /// absorbing late callbacks afterwards.
    #[test]
    fn anomaly_arm_serde_round_trips() {
        use platform::ica::ErrorResponse as ICAErrorResponse;

        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (_response, parked) = continued(after_first_ack(querier).on_remote_error(
            ICAErrorResponse::from(String::from("swap reverted")),
            querier,
            testing::mock_env(),
        ));
        assert!(matches!(parked, State::SlippageAnomaly(_)));

        let serialized = sdk::cosmwasm_std::to_json_vec(&parked).expect("a serializable arm");
        let restored: State<MockSpec> =
            sdk::cosmwasm_std::from_json(&serialized).expect("the anomaly arm should round-trip");
        assert_eq!(
            serialized,
            sdk::cosmwasm_std::to_json_vec(&restored).expect("a serializable arm"),
        );
        assert!(matches!(restored, State::SlippageAnomaly(_)));

        let (_response, still_parked) = continued(restored.on_remote_response(
            payload(&coin_out(40)),
            querier,
            testing::mock_env(),
        ));
        assert!(matches!(still_parked, State::SlippageAnomaly(_)));
    }

    fn continued(res: HandlerResult<State<MockSpec>>) -> (MessageResponse, State<MockSpec>) {
        match res {
            HandlerResult::Continue(Ok(resp)) => (resp.response, resp.next_state),
            HandlerResult::Continue(Err(err)) => panic!("expected a continuation, got {err}"),
            HandlerResult::Finished(_total) => panic!("expected a continuation, got a finish"),
        }
    }

    fn finished(res: HandlerResult<State<MockSpec>>) -> CoinDTO<OutG> {
        match res {
            HandlerResult::Finished(total) => total,
            HandlerResult::Continue(_resp) => panic!("expected a finish, got a continuation"),
        }
    }

    fn after_first_ack(querier: QuerierWrapper<'_>) -> State<MockSpec> {
        let (_response, state) =
            continued(start(spec3(), &testing::mock_env(), querier).map_into());
        continued(state.on_remote_response(payload(&coin_out(30)), querier, testing::mock_env())).1
    }

    fn spec3() -> MockSpec {
        MockSpec::new(vec![coin_in(100), coin_out(50), coin_in(70)])
    }

    fn leg_response(
        leg: &CoinDTO<OutG>,
        min_out: &CoinDTO<OutG>,
        total: &CoinDTO<OutG>,
    ) -> MessageResponse {
        MessageResponse::messages_with_event(
            mock::swap_request(leg, min_out).expect("a valid swap request"),
            Emitter::of_type(mock::LABEL).emit_coin_dto("total-out", total),
        )
    }

    fn payload(coin: &CoinDTO<OutG>) -> Binary {
        sdk::cosmwasm_std::to_json_binary(coin).expect("a serializable coin")
    }

    fn coin_out(amount: Amount) -> CoinDTO<OutG> {
        Coin::<SuperGroupTestC1>::new(amount).into()
    }

    fn coin_in(amount: Amount) -> CoinDTO<OutG> {
        Coin::<SuperGroupTestC2>::new(amount).into()
    }

    fn min_out() -> CoinDTO<OutG> {
        coin_out(1)
    }
}
