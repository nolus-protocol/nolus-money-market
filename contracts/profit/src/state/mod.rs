use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use dex::{
    ConnectionParams, ContinueResult, Handler, Response as DexResponse, Result as DexResult,
    StateLocalOut,
};
use platform::state_machine::{self, Response as StateMachineResponse};
use sdk::{
    cosmwasm_std::{Binary, Deps, Env, Storage},
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

pub(crate) trait ConfigManagement
where
    Self: Sized,
{
    fn try_update_config(self, cadence_hours: CadenceHours) -> ContractResult<Self>;

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
    fn try_update_config(self, cadence_hours: CadenceHours) -> ContractResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.try_update_config(cadence_hours).map(Into::into)
            }
            StateEnum::OpenIca(ica) => ica.try_update_config(cadence_hours).map(Into::into),
            StateEnum::Idle(idle) => idle.try_update_config(cadence_hours).map(Into::into),
            StateEnum::BuyBack(buy_back) => {
                buy_back.try_update_config(cadence_hours).map(Into::into)
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
