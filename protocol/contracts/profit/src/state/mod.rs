use std::fmt::{Display, Formatter};

use finance::duration::Duration;
use serde::{Deserialize, Serialize};

use dex::{
    ContinueResult, Contract, Handler, Response as DexResponse, Result as SwapDecision, StateDrain,
    StateFundRemote,
};
use platform::{
    batch::Batch,
    ica::ErrorResponse as ICAErrorResponse,
    state_machine::{self, Response as StateMachineResponse},
};
use remote_profit::{
    callback::{RemoteOperationOutcome, RemoteProfitCallback},
    response::WireOperationResponse,
};
use sdk::cosmwasm_std::{
    self, Binary, Env, MessageInfo, QuerierWrapper, Reply as CwReply, Storage,
};

use crate::{CadenceHours, error::ContractError, msg::ConfigResponse, result::ContractResult};

pub(crate) use self::config::{Config, VaultConfig};
use self::{
    buy_back::BuyBack, drain::ProfitDrain, idle::Idle, open_profit::OpenProfit,
    resp_delivery::ForwardToDexEntry,
};
use finance::instant::Instant;
use sdk::cw_storage_plus::Item;

mod arrival;
mod buy_back;
mod config;
mod drain;
mod idle;
mod open_profit;
mod resp_delivery;

const STATE: Item<State> = Item::new("contract_state");

type FundRemote = StateFundRemote<BuyBack, ForwardToDexEntry>;
type Drain = StateDrain<ProfitDrain>;

pub(crate) trait ConfigManagement
where
    Self: Sized,
{
    fn try_update_config(
        self,
        _: Instant,
        _: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        Err(ContractError::unsupported_operation(
            "Configuration changes are not allowed in this state!",
        ))
    }
}

#[derive(Serialize, Deserialize)]
enum StateEnum {
    /// The establishment state: the drain vault is being (or has been)
    /// instantiated and the `OpenProfit` packet awaits its acknowledgment that
    /// carries the Solana profit authority. No cycle runs until it transitions
    /// to `Idle`.
    OpenProfit(OpenProfit),
    Idle(Idle),
    FundRemote(FundRemote),
    Drain(Drain),
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct State(StateEnum);

impl From<Idle> for State {
    fn from(value: Idle) -> Self {
        Self(StateEnum::Idle(value))
    }
}

impl From<OpenProfit> for State {
    fn from(value: OpenProfit) -> Self {
        Self(StateEnum::OpenProfit(value))
    }
}

impl From<FundRemote> for State {
    fn from(value: FundRemote) -> Self {
        Self(StateEnum::FundRemote(value))
    }
}

impl From<Drain> for State {
    fn from(value: Drain) -> Self {
        Self(StateEnum::Drain(value))
    }
}

impl ConfigManagement for State {
    fn try_update_config(
        self,
        now: Instant,
        cadence_hours: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        match self.0 {
            StateEnum::Idle(idle) => idle
                .try_update_config(now, cadence_hours)
                .map(state_machine::from),
            // The establishment state and a live remote cycle hold no config to
            // mutate; the cadence is re-armed on return to `Idle`.
            StateEnum::OpenProfit(_) | StateEnum::FundRemote(_) | StateEnum::Drain(_) => {
                Err(ContractError::unsupported_operation(
                    "Configuration changes are not allowed in this state!",
                ))
            }
        }
    }
}

impl State {
    /// Build the establishment state over the freshly-built config. The drain
    /// vault is instantiated and the `OpenProfit` packet emitted by the
    /// contract's instantiate/reply path; this only seeds the stored state.
    pub fn start(config: Config) -> Self {
        State(StateEnum::OpenProfit(OpenProfit::new(config)))
    }

    /// Resolve the `Instantiate2` success reply: only valid in the
    /// establishment state, it verifies the vault address (FM2 fail-closed) and
    /// emits the `OpenProfit` packet.
    pub fn on_vault_instantiated(
        self,
        instantiated: sdk::cosmwasm_std::Addr,
    ) -> ContractResult<(Batch, Self)> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.confirm_vault_and_open(instantiated),
            StateEnum::Idle(_) | StateEnum::FundRemote(_) | StateEnum::Drain(_) => {
                Err(ContractError::unsupported_operation(
                    "an instantiate reply is only expected during establishment",
                ))
            }
        }
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        STATE.load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        STATE.save(storage, self).map_err(Into::into)
    }

    /// Authorise then split an inbound `RemoteProfitCallback` to the leg that
    /// scheduled the in-flight remote operation. Mirrors the lease's
    /// `on_remote_lease_callback`: an authorised callback is delivered to the
    /// `on_remote_*` entry points; the absorbing `Handler` defaults swallow a
    /// callback that reaches a leg holding nothing in flight.
    pub fn on_remote_profit_callback(
        self,
        callback: RemoteProfitCallback,
        info: MessageInfo,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<State> {
        match self.authz_remote_callback(querier, &info) {
            Ok(()) => self.deliver_remote_callback(callback, querier, env),
            Err(err) => SwapDecision::Continue(Err(err)),
        }
    }

    fn deliver_remote_callback(
        self,
        callback: RemoteProfitCallback,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<State> {
        let RemoteProfitCallback { nonce, outcome } = callback;
        match outcome {
            RemoteOperationOutcome::OperationOk(response) => {
                self.deliver_remote_ok(&response, nonce, querier, env)
            }
            RemoteOperationOutcome::OperationErr(message) => self.on_remote_error(
                ICAErrorResponse::from(message.as_str().to_owned()),
                nonce,
                querier,
                env,
            ),
            RemoteOperationOutcome::OperationTimeout => self.on_remote_timeout(nonce, querier, env),
        }
    }

    fn deliver_remote_ok(
        self,
        response: &WireOperationResponse,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<State> {
        match cosmwasm_std::to_json_binary(response) {
            Ok(data) => self.on_remote_response(data, nonce, querier, env),
            Err(err) => SwapDecision::Finished(Err(ContractError::from(err))),
        }
    }
}

impl Contract for State {
    type StateResponse = ConfigResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        match self.0 {
            StateEnum::OpenProfit(open) => open.state(now, due_projection, querier),
            StateEnum::Idle(idle) => idle.state(now, due_projection, querier),
            StateEnum::FundRemote(fund) => fund.state(now, due_projection, querier),
            StateEnum::Drain(drain) => drain.state(now, due_projection, querier),
        }
    }
}

impl Handler for State {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &MessageInfo,
    ) -> dex::DexResult<()> {
        match &self.0 {
            StateEnum::OpenProfit(open) => open.authz_remote_callback(querier, info),
            StateEnum::Idle(idle) => idle.authz_remote_callback(querier, info),
            StateEnum::FundRemote(fund) => fund.authz_remote_callback(querier, info),
            StateEnum::Drain(drain) => drain.authz_remote_callback(querier, info),
        }
    }

    fn on_response(
        self,
        data: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_response(data, querier, env).map_into(),
            StateEnum::Idle(idle) => idle.on_response(data, querier, env).map_into(),
            StateEnum::FundRemote(fund) => fund.on_response(data, querier, env).map_into(),
            StateEnum::Drain(drain) => drain.on_response(data, querier, env).map_into(),
        }
    }

    fn on_error(
        self,
        response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_error(response, querier, env).map_into(),
            StateEnum::Idle(idle) => idle.on_error(response, querier, env).map_into(),
            StateEnum::FundRemote(fund) => fund.on_error(response, querier, env).map_into(),
            StateEnum::Drain(drain) => drain.on_error(response, querier, env).map_into(),
        }
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_timeout(querier, env).map(state_machine::from),
            StateEnum::Idle(idle) => idle.on_timeout(querier, env).map(state_machine::from),
            StateEnum::FundRemote(fund) => fund.on_timeout(querier, env).map(state_machine::from),
            StateEnum::Drain(drain) => drain.on_timeout(querier, env).map(state_machine::from),
        }
    }

    fn on_inner(self, querier: QuerierWrapper<'_>, env: Env) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_inner(querier, env).map_into(),
            StateEnum::Idle(idle) => idle.on_inner(querier, env).map_into(),
            StateEnum::FundRemote(fund) => fund.on_inner(querier, env).map_into(),
            StateEnum::Drain(drain) => drain.on_inner(querier, env).map_into(),
        }
    }

    /// FM3: kept and re-routed over the remote arms. The funding leg's
    /// response-delivery indirection (`ForwardToDexEntry` → `DexCallback`)
    /// continues through this entry point.
    fn on_inner_continue(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open
                .on_inner_continue(querier, env)
                .map(state_machine::from),
            StateEnum::Idle(idle) => idle
                .on_inner_continue(querier, env)
                .map(state_machine::from),
            StateEnum::FundRemote(fund) => fund
                .on_inner_continue(querier, env)
                .map(state_machine::from),
            StateEnum::Drain(drain) => drain
                .on_inner_continue(querier, env)
                .map(state_machine::from),
        }
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env, info: &MessageInfo) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.heal(querier, env, info).map_into(),
            StateEnum::Idle(idle) => idle.heal(querier, env, info).map_into(),
            StateEnum::FundRemote(fund) => fund.heal(querier, env, info).map_into(),
            StateEnum::Drain(drain) => drain.heal(querier, env, info).map_into(),
        }
    }

    fn reply(self, querier: QuerierWrapper<'_>, env: Env, msg: CwReply) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.reply(querier, env, msg).map(state_machine::from),
            StateEnum::Idle(idle) => idle.reply(querier, env, msg).map(state_machine::from),
            StateEnum::FundRemote(fund) => fund.reply(querier, env, msg).map(state_machine::from),
            StateEnum::Drain(drain) => drain.reply(querier, env, msg).map(state_machine::from),
        }
    }

    fn on_time_alarm(
        self,
        querier: QuerierWrapper<'_>,
        env: Env,
        info: MessageInfo,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_time_alarm(querier, env, info).map_into(),
            StateEnum::Idle(idle) => idle.on_time_alarm(querier, env, info).map_into(),
            StateEnum::FundRemote(fund) => fund.on_time_alarm(querier, env, info).map_into(),
            StateEnum::Drain(drain) => drain.on_time_alarm(querier, env, info).map_into(),
        }
    }

    fn on_remote_response(
        self,
        data: Binary,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open
                .on_remote_response(data, nonce, querier, env)
                .map_into(),
            StateEnum::Idle(idle) => idle
                .on_remote_response(data, nonce, querier, env)
                .map_into(),
            StateEnum::FundRemote(fund) => fund
                .on_remote_response(data, nonce, querier, env)
                .map_into(),
            StateEnum::Drain(drain) => drain
                .on_remote_response(data, nonce, querier, env)
                .map_into(),
        }
    }

    fn on_remote_error(
        self,
        response: ICAErrorResponse,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open
                .on_remote_error(response, nonce, querier, env)
                .map_into(),
            StateEnum::Idle(idle) => idle
                .on_remote_error(response, nonce, querier, env)
                .map_into(),
            StateEnum::FundRemote(fund) => fund
                .on_remote_error(response, nonce, querier, env)
                .map_into(),
            StateEnum::Drain(drain) => drain
                .on_remote_error(response, nonce, querier, env)
                .map_into(),
        }
    }

    fn on_remote_timeout(
        self,
        nonce: u64,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> SwapDecision<Self> {
        match self.0 {
            StateEnum::OpenProfit(open) => open.on_remote_timeout(nonce, querier, env).map_into(),
            StateEnum::Idle(idle) => idle.on_remote_timeout(nonce, querier, env).map_into(),
            StateEnum::FundRemote(fund) => fund.on_remote_timeout(nonce, querier, env).map_into(),
            StateEnum::Drain(drain) => drain.on_remote_timeout(nonce, querier, env).map_into(),
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            StateEnum::OpenProfit(open) => Display::fmt(&open, f),
            StateEnum::Idle(idle) => Display::fmt(&idle, f),
            StateEnum::FundRemote(fund) => Display::fmt(&fund, f),
            StateEnum::Drain(drain) => Display::fmt(&drain, f),
        }
    }
}
