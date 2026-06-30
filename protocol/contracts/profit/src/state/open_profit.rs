use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use cw_time::IntoInstant;
use dex::{
    Contract, DexResult, Enterable, Error as DexError, Handler, Response as DexResponse,
    Result as SwapDecision,
};
use finance::{duration::Duration, instant::Instant};
use platform::{batch::Batch, message::Response as PlatformResponse};
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

/// A non-`OpenProfit` acknowledgment cannot have resolved the establishment
/// packet — the only operation this state emits.
const NON_OPEN_PROFIT_RESPONSE: &str = "non-open-profit operation response";

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

    fn on_remote_response(
        self,
        data: sdk::cosmwasm_std::Binary,
        _nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        let decoded = cosmwasm_std::from_json::<OperationResponse>(data.as_slice())
            .map_err(ContractError::from);
        match decoded {
            Ok(OperationResponse::OpenProfit(response)) => {
                SwapDecision::Finished(self.learn_authority(&response, &env, querier))
            }
            Ok(_other) => SwapDecision::Finished(Err(ContractError::unsupported_operation(
                NON_OPEN_PROFIT_RESPONSE,
            ))),
            Err(err) => SwapDecision::Finished(Err(err)),
        }
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
