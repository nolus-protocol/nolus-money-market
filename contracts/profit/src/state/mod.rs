use std::fmt::{Display, Formatter};

use finance::duration::Duration;
use platform::{batch::Batch, message::Response as PlatformResponse};
use serde::{Deserialize, Serialize};

use dex::{
    ConnectionParams, ContinueResult, Handler, Response as DexResponse, Result as DexResult,
    StateLocalOut,
};
use platform::state_machine::{self, Response as StateMachineResponse};
use sdk::{
    cosmwasm_std::{Binary, Deps, DepsMut, Env, Reply as CwReply, Storage, Timestamp},
    cw_storage_plus::Item,
};

use crate::{
    error::ContractError, msg::ConfigResponse, result::ContractResult, typedefs::CadenceHours,
};

pub(crate) use self::config::Config;
use self::{
    buy_back::BuyBack,
    idle::Idle,
    open_ica::OpenIca,
    open_transfer_channel::OpenTransferChannel,
    resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
};

mod buy_back;
mod config;
mod idle;
mod open_ica;
mod open_transfer_channel;
mod resp_delivery;

const STATE: Item<'static, State> = Item::new("contract_state");

type IcaConnector = dex::IcaConnector<OpenIca, ContractResult<DexResponse<Idle>>>;

pub(crate) struct StateAndResponse<T> {
    pub state: T,
    pub response: platform::message::Response,
}

impl<T> StateAndResponse<T> {
    pub fn map_state<U>(self) -> StateAndResponse<U>
    where
        T: Into<U>,
    {
        StateAndResponse {
            state: self.state.into(),
            response: self.response,
        }
    }
}

pub(crate) trait ConfigManagement
where
    Self: Sized,
{
    fn with_config<F>(self, f: F) -> ContractResult<StateAndResponse<Self>>
    where
        F: FnOnce(Config) -> ContractResult<StateAndResponse<Config>>;

    fn try_update_config(
        self,
        now: Timestamp,
        cadence_hours: CadenceHours,
    ) -> ContractResult<StateAndResponse<Self>> {
        self.with_config(|config: Config| {
            config
                .time_alarms()
                .setup_alarm(now + Duration::from_hours(cadence_hours))
                .map(|messages: Batch| StateAndResponse {
                    state: config.update(cadence_hours),
                    response: PlatformResponse::messages_only(messages),
                })
                .map_err(Into::into)
        })
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse>;
}

pub(crate) trait SetupDexHandler
where
    Self: Sized,
{
    type State: Into<State>;

    fn setup_dex(
        self,
        _: Deps<'_>,
        _: Env,
        _: ConnectionParams,
    ) -> ContractResult<StateMachineResponse<Self::State>> {
        Err(ContractError::UnsupportedOperation(String::from(
            "Dex is already setup!",
        )))
    }
}

#[derive(Serialize, Deserialize)]
enum StateEnum {
    OpenTransferChannel(OpenTransferChannel),
    OpenIca(IcaConnector),
    Idle(Idle),
    BuyBack(StateLocalOut<BuyBack, ForwardToDexEntry, ForwardToDexEntryContinue>),
}

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct State(StateEnum);

impl ConfigManagement for State {
    fn with_config<F>(self, f: F) -> ContractResult<StateAndResponse<Self>>
    where
        F: FnOnce(Config) -> ContractResult<StateAndResponse<Config>>,
    {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.with_config(f).map(StateAndResponse::map_state)
            }
            StateEnum::OpenIca(ica) => ica.with_config(f).map(StateAndResponse::map_state),
            StateEnum::Idle(idle) => idle.with_config(f).map(StateAndResponse::map_state),
            StateEnum::BuyBack(buy_back) => {
                buy_back.with_config(f).map(StateAndResponse::map_state)
            }
        }
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        match &self.0 {
            StateEnum::OpenTransferChannel(transfer) => transfer.try_query_config(),
            StateEnum::OpenIca(ica) => ica.try_query_config(),
            StateEnum::Idle(idle) => idle.try_query_config(),
            StateEnum::BuyBack(buy_back) => buy_back.try_query_config(),
        }
    }
}

impl State {
    pub fn new(config: Config) -> Self {
        Self(StateEnum::OpenTransferChannel(OpenTransferChannel::new(
            config,
        )))
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        STATE.load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        STATE.save(storage, self).map_err(Into::into)
    }
}

impl From<OpenTransferChannel> for State {
    fn from(value: OpenTransferChannel) -> Self {
        Self(StateEnum::OpenTransferChannel(value))
    }
}

impl From<IcaConnector> for State {
    fn from(value: IcaConnector) -> Self {
        Self(StateEnum::OpenIca(value))
    }
}

impl From<Idle> for State {
    fn from(value: Idle) -> Self {
        Self(StateEnum::Idle(value))
    }
}

impl From<StateLocalOut<BuyBack, ForwardToDexEntry, ForwardToDexEntryContinue>> for State {
    fn from(value: StateLocalOut<BuyBack, ForwardToDexEntry, ForwardToDexEntryContinue>) -> Self {
        Self(StateEnum::BuyBack(value))
    }
}

impl SetupDexHandler for State {
    type State = Self;

    fn setup_dex(
        self,
        deps: Deps<'_>,
        env: Env,
        connection: ConnectionParams,
    ) -> ContractResult<StateMachineResponse<Self>> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => transfer
                .setup_dex(deps, env, connection)
                .map(state_machine::from),
            StateEnum::OpenIca(ica) => ica
                .setup_dex(deps, env, connection)
                .map(state_machine::from),
            StateEnum::Idle(idle) => idle
                .setup_dex(deps, env, connection)
                .map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back
                .setup_dex(deps, env, connection)
                .map(state_machine::from),
        }
    }
}

impl Handler for State {
    type Response = State;
    type SwapResult = ContractResult<DexResponse<State>>;

    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.on_open_ica(counterparty_version, deps, env)
            }
            StateEnum::OpenIca(ica) => ica.on_open_ica(counterparty_version, deps, env),
            StateEnum::Idle(idle) => idle.on_open_ica(counterparty_version, deps, env),
            StateEnum::BuyBack(buy_back) => buy_back
                .on_open_ica(counterparty_version, deps, env)
                .map(state_machine::from),
        }
    }

    fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.on_response(data, deps, env).map_into()
            }
            StateEnum::OpenIca(ica) => ica.on_response(data, deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_response(data, deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_response(data, deps, env).map_into(),
        }
    }

    fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => transfer.on_error(deps, env),
            StateEnum::OpenIca(ica) => ica.on_error(deps, env),
            StateEnum::Idle(idle) => idle.on_error(deps, env),
            StateEnum::BuyBack(buy_back) => buy_back.on_error(deps, env).map(state_machine::from),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.on_timeout(deps, env).map(state_machine::from)
            }
            StateEnum::OpenIca(ica) => ica.on_timeout(deps, env).map(state_machine::from),
            StateEnum::Idle(idle) => idle.on_timeout(deps, env).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back.on_timeout(deps, env).map(state_machine::from),
        }
    }

    fn on_inner(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => transfer.on_inner(deps, env).map_into(),
            StateEnum::OpenIca(ica) => ica.on_inner(deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_inner(deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_inner(deps, env).map_into(),
        }
    }

    fn on_inner_continue(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => transfer
                .on_inner_continue(deps, env)
                .map(state_machine::from),
            StateEnum::OpenIca(ica) => ica.on_inner_continue(deps, env).map(state_machine::from),
            StateEnum::Idle(idle) => idle.on_inner_continue(deps, env).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back
                .on_inner_continue(deps, env)
                .map(state_machine::from),
        }
    }

    fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: CwReply) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.reply(deps, env, msg).map(state_machine::from)
            }
            StateEnum::OpenIca(ica) => ica.reply(deps, env, msg).map(state_machine::from),
            StateEnum::Idle(idle) => idle.reply(deps, env, msg).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back.reply(deps, env, msg).map(state_machine::from),
        }
    }

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.on_time_alarm(deps, env).map_into()
            }
            StateEnum::OpenIca(ica) => ica.on_time_alarm(deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_time_alarm(deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_time_alarm(deps, env).map_into(),
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            StateEnum::OpenTransferChannel(transfer) => Display::fmt(&transfer, f),
            StateEnum::OpenIca(ica) => Display::fmt(&ica, f),
            StateEnum::Idle(idle) => Display::fmt(&idle, f),
            StateEnum::BuyBack(buy_back) => Display::fmt(&buy_back, f),
        }
    }
}
