use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use cw_time::IntoInstant;
use dex::{
    Contract, DexResult, Enterable, Error as DexError, Handler, Response as DexResponse,
    Result as SwapDecision,
};
use finance::{duration::Duration, instant::Instant};
use platform::{
    batch::{Batch, Emit, Emitter},
    message::Response as PlatformResponse,
};
use remote_profit::{
    msg::{NolusReceiver, OpenProfitParams},
    response::{OpenProfitResponse, OperationResponse},
    stub::{ControllerInnerMessage, Factory as ControllerFactory},
};
use sdk::cosmwasm_std::{self, Addr, Env, MessageInfo, QuerierWrapper};

use crate::{error::ContractError, msg::ConfigResponse, result::ContractResult};

use super::{Config, State, StateEnum, idle::Idle};

/// The replay-guard ordinal stamped on the singleton open. The profit is a
/// singleton selected by port/domain/channel, so the first establishment uses
/// the first ordinal; a fresh-instance cutover (the only re-establishment path)
/// is governance-driven and deploys a new contract.
const INSTANCE_ORDINAL: u16 = 0;

/// The establishment event surfacing a bad-ack absorb or an operator heal, so
/// monitoring can tell a wedged-then-recovered establishment apart from a
/// normal cycle.
const EVENT_TYPE_ESTABLISHMENT: &str = "profit-establishment";
const EVENT_KEY_ABSORBED: &str = "absorbed";
const EVENT_KEY_HEAL: &str = "heal";
const EVENT_VALUE_REEMIT: &str = "re-emit";
/// A decodable acknowledgment carrying a non-`OpenProfit` variant.
const ABSORB_UNEXPECTED_VARIANT: &str = "unexpected-response-variant";
/// An acknowledgment payload that does not decode into an operation response.
const ABSORB_UNDECODABLE: &str = "undecodable-response";

/// The establishment state.
///
/// Seeded at instantiate. The drain vault is instantiated via `Instantiate2`;
/// its address is verified in [`Handler::reply`], which then emits the
/// `OpenProfit` packet committing the verified vault as the drain receiver. The
/// `OpenProfit` acknowledgment ([`Handler::on_remote_response`]) carries the
/// Solana profit authority, which is stored before transitioning to `Idle`.
#[derive(Serialize, Deserialize)]
pub(super) struct OpenProfit {
    config: Config,
}

impl OpenProfit {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Verify the `Instantiate2`-reported vault address matches the precomputed
    /// one (FM2 fail-closed), then emit the `OpenProfit` packet committing that
    /// verified vault as the store-once drain receiver. Invoked from the
    /// contract reply entry point, which owns the `Api` the address decode needs.
    pub fn confirm_vault_and_open(self, instantiated: Addr) -> ContractResult<(Batch, State)> {
        if instantiated != *self.config.drain_vault() {
            return Err(ContractError::DifferentInstantiatedAddress {
                reported: instantiated,
                expected: self.config.drain_vault().clone(),
            });
        }
        self.open_profit_msg()
            .map(|batch| (batch, State(StateEnum::OpenProfit(self))))
    }

    /// The `OpenProfit` packet committing the precomputed drain vault as the
    /// store-once drain receiver.
    fn open_profit_msg(&self) -> ContractResult<Batch> {
        NolusReceiver::new(self.config.drain_vault().as_str())
            .map_err(ContractError::from)
            .and_then(|receiver| {
                ControllerFactory::new(self.config.remote_profit_controller())
                    .open(
                        OpenProfitParams::new(INSTANCE_ORDINAL, receiver),
                        OpenProfitParams::TIMEOUT,
                        |params, timeout| ControllerExecuteMsg::OpenProfit { params, timeout },
                    )
                    .map_err(Into::into)
            })
    }

    fn learn_authority(
        self,
        response: &OpenProfitResponse,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<DexResponse<State>> {
        let config = self
            .config
            .with_profit_authority(response.remote_profit_id.clone());
        let idle = Idle::new(config);
        idle.enter(env.block.time.into_instant(), querier)
            .map(|batch: Batch| {
                DexResponse::<State>::from(
                    PlatformResponse::messages_only(batch),
                    State(StateEnum::Idle(idle)),
                )
            })
            .map_err(Into::into)
    }

    /// Commit the acknowledgment (so the relayer stops redelivering) while
    /// staying in the establishment state and surfacing `event`. Shared by the
    /// bad-ack absorb and the operator heal re-emission.
    fn stay_open(self, batch: Batch, event: Emitter) -> SwapDecision<Self> {
        SwapDecision::Continue(Ok(DexResponse::<State>::from(
            PlatformResponse::messages_with_event(batch, event),
            State(StateEnum::OpenProfit(self)),
        )))
    }

    /// Absorb a bad establishment acknowledgment — a decodable non-`OpenProfit`
    /// variant or an undecodable payload. Erroring would revert the controller's
    /// ack and loop the relayer on the same one-shot packet; absorbing commits
    /// the ack and leaves the profit in `OpenProfit`, recoverable via
    /// [`Handler::heal`].
    fn absorb(self, reason: &str) -> SwapDecision<Self> {
        let event = Emitter::of_type(EVENT_TYPE_ESTABLISHMENT).emit(EVENT_KEY_ABSORBED, reason);
        self.stay_open(Batch::default(), event)
    }

    /// Re-emit the one-shot establishment packet to re-solicit the
    /// acknowledgment, recovering a profit wedged in `OpenProfit` by an absorbed
    /// bad ack.
    fn reemit_establishment(self) -> SwapDecision<Self> {
        match self.open_profit_msg() {
            Ok(batch) => {
                let event = Emitter::of_type(EVENT_TYPE_ESTABLISHMENT)
                    .emit(EVENT_KEY_HEAL, EVENT_VALUE_REEMIT);
                self.stay_open(batch, event)
            }
            Err(err) => SwapDecision::Finished(Err(err)),
        }
    }
}

impl Handler for OpenProfit {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    /// The `OpenProfit` acknowledgment arrives as a controller callback, so the
    /// establishment state authorises the same controller every cycle will.
    fn authz_remote_callback(
        &self,
        _querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> DexResult<()> {
        access_control::check(
            &access_control::permissions::SingleUserPermission::new(
                self.config.remote_profit_controller(),
            ),
            info,
        )
        .map_err(DexError::Unauthorized)
    }

    /// A decodable non-`OpenProfit` variant and an undecodable payload are both
    /// absorbed rather than erroring: the establishment ack rides the callback
    /// path, so an error would revert the controller's ack and loop the relayer
    /// forever on the same malformed packet. A well-formed `OpenProfit` ack runs
    /// the regular flow and lets any downstream failure propagate.
    fn on_remote_response(
        self,
        data: sdk::cosmwasm_std::Binary,
        _nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match cosmwasm_std::from_json::<OperationResponse>(data.as_slice()) {
            Ok(OperationResponse::OpenProfit(response)) => {
                SwapDecision::Finished(self.learn_authority(&response, &env, querier))
            }
            Ok(_other) => self.absorb(ABSORB_UNEXPECTED_VARIANT),
            Err(_undecodable) => self.absorb(ABSORB_UNDECODABLE),
        }
    }

    /// The one operator recovery out of a wedged establishment: an absorbed bad
    /// ack consumed the one-shot `OpenProfit` packet without transitioning, so
    /// no further callback will arrive. Re-emitting the packet re-solicits the
    /// acknowledgment. Permissionless, like the remote-swap heal — the
    /// controller still authorises the callback it answers with.
    fn heal(
        self,
        _querier: QuerierWrapper<'_>,
        _env: Env,
        _info: &MessageInfo,
    ) -> SwapDecision<Self> {
        self.reemit_establishment()
    }
}

impl Contract for OpenProfit {
    type StateResponse = ConfigResponse;

    fn state(
        self,
        _now: Instant,
        _due_projection: Duration,
        _querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        ConfigResponse {
            cadence_hours: self.config.cadence_hours(),
        }
    }
}

impl Display for OpenProfit {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_str("OpenProfit")
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ControllerExecuteMsg {
    OpenProfit {
        params: OpenProfitParams,
        timeout: Duration,
    },
}

impl ControllerInnerMessage for ControllerExecuteMsg {}

#[cfg(all(feature = "internal.test.contract", test))]
mod tests {
    use dex::{Account, ConnectionParams, Handler, Ics20Channel, Result as SwapDecision};
    use platform::response;
    use remote_profit::response::{
        OpenProfitResponse, OperationResponse, RemoteProfitId, TransferOutResponse,
    };
    use sdk::{
        cosmwasm_ext::Response as CwResponse,
        cosmwasm_std::{
            self, Addr, Binary, Event, MessageInfo, QuerierWrapper,
            testing::{self, MockQuerier},
        },
    };
    use timealarms::stub::TimeAlarmsRef;

    use crate::state::VaultConfig;

    use super::{Config, OpenProfit, State};

    /// A valid bech32 Nolus address so the establishment message the heal
    /// re-emits builds its `NolusReceiver` successfully.
    const DRAIN_VAULT: &str = "nolus1mf6ptkssddfmxvhdx0ech0k03ktp6kf9yk59renau2gvht3nq2gqkxgywu";
    const CONTROLLER: &str = "controller";
    const CADENCE_HOURS: u16 = 24;

    /// A decodable non-`OpenProfit` acknowledgment is committed, not errored, and
    /// leaves the profit in the establishment state so the relayer stops
    /// redelivering.
    #[test]
    fn bad_variant_ack_is_absorbed_and_stays_open() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, next) = continued(open().on_remote_response(
            ack(&OperationResponse::TransferOut(TransferOutResponse {})),
            NONCE,
            querier,
            testing::mock_env(),
        ));

        assert_eq!("OpenProfit", next.to_string());
        assert_absorbed(super::ABSORB_UNEXPECTED_VARIANT, &response);
    }

    /// An undecodable acknowledgment payload is absorbed the same way.
    #[test]
    fn undecodable_ack_is_absorbed_and_stays_open() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, next) = continued(open().on_remote_response(
            Binary::from(b"not an operation response".as_slice()),
            NONCE,
            querier,
            testing::mock_env(),
        ));

        assert_eq!("OpenProfit", next.to_string());
        assert_absorbed(super::ABSORB_UNDECODABLE, &response);
    }

    /// An operator heal re-emits the one establishment packet and stays in the
    /// establishment state, so a profit wedged by an absorbed bad ack recovers.
    #[test]
    fn heal_reemits_the_establishment_packet() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let (response, next) = continued(open().heal(querier, testing::mock_env(), &healer()));

        assert_eq!("OpenProfit", next.to_string());
        assert_eq!(1, response.messages.len());
        assert_eq!(1, response.events.len());
        assert_eq!(
            Event::new(super::EVENT_TYPE_ESTABLISHMENT)
                .add_attribute(super::EVENT_KEY_HEAL, super::EVENT_VALUE_REEMIT),
            response.events[0]
        );
    }

    /// The happy path is unchanged: a well-formed `OpenProfit` acknowledgment
    /// learns the authority and transitions to `Idle`.
    #[test]
    fn open_profit_ack_transitions_to_idle() {
        let mock_querier = MockQuerier::default();
        let querier = QuerierWrapper::new(&mock_querier);

        let next = finished(open().on_remote_response(
            ack(&OperationResponse::OpenProfit(OpenProfitResponse {
                remote_profit_id: profit_id(),
            })),
            NONCE,
            querier,
            testing::mock_env(),
        ));

        assert_eq!("Idle", next.to_string());
    }

    const NONCE: u64 = 1;

    fn assert_absorbed(reason: &str, response: &CwResponse) {
        assert_eq!(0, response.messages.len());
        assert_eq!(1, response.events.len());
        assert_eq!(
            Event::new(super::EVENT_TYPE_ESTABLISHMENT)
                .add_attribute(super::EVENT_KEY_ABSORBED, reason),
            response.events[0]
        );
    }

    fn continued(res: SwapDecision<OpenProfit>) -> (CwResponse, State) {
        match res {
            SwapDecision::Continue(Ok(resp)) => (
                response::response_only_messages(resp.response),
                resp.next_state,
            ),
            SwapDecision::Continue(Err(err)) => panic!("expected a continuation, got error {err}"),
            SwapDecision::Finished(_res) => panic!("expected a continuation, got a finish"),
        }
    }

    fn finished(res: SwapDecision<OpenProfit>) -> State {
        match res {
            SwapDecision::Finished(Ok(resp)) => resp.next_state,
            SwapDecision::Finished(Err(err)) => panic!("expected a finish, got error {err}"),
            SwapDecision::Continue(_res) => panic!("expected a finish, got a continuation"),
        }
    }

    fn open() -> OpenProfit {
        OpenProfit::new(config())
    }

    fn config() -> Config {
        Config::new(
            CADENCE_HOURS,
            Addr::unchecked("treasury"),
            oracle_platform::OracleRef::unchecked(Addr::unchecked("oracle")),
            TimeAlarmsRef::unchecked("timealarms"),
            Account::funding(
                Addr::unchecked("profit"),
                ConnectionParams {
                    connection_id: "connection-0".to_owned(),
                    transfer_channel: Ics20Channel {
                        local_endpoint: "channel-0".to_owned(),
                        remote_endpoint: "channel-2048".to_owned(),
                    },
                },
            ),
            Addr::unchecked(CONTROLLER),
            VaultConfig {
                code_id: cosmwasm_std::from_json(b"3").expect("a valid code id"),
                address: Addr::unchecked(DRAIN_VAULT),
            },
        )
    }

    fn healer() -> MessageInfo {
        MessageInfo {
            sender: Addr::unchecked(CONTROLLER),
            funds: vec![],
        }
    }

    fn ack(response: &OperationResponse) -> Binary {
        cosmwasm_std::to_json_binary(response).expect("the response serializes")
    }

    fn profit_id() -> RemoteProfitId {
        RemoteProfitId::new("So1RayProfit").expect("a base58 profit id")
    }
}
