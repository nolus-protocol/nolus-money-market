use std::fmt::{Display, Formatter};

use currencies::PaymentGroup;
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
use swap::Impl;

use crate::{
    error::ContractError, msg::ConfigResponse, result::ContractResult, typedefs::CadenceHours,
};

pub(crate) use self::config::Config;
use self::{
    buy_back::BuyBack,
    idle::Idle,
    open_ica::OpenIca,
    resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
};

mod buy_back;
mod config;
mod idle;
mod open_ica;
mod resp_delivery;

const STATE: Item<'static, State> = Item::new("contract_state");

type IcaConnector = dex::IcaConnector<OpenIca, ContractResult<DexResponse<Idle>>>;
type ProfitCurrencies = PaymentGroup;
type SwapClient = Impl;

pub(crate) trait ConfigManagement
where
    Self: Sized,
{
    fn try_update_config(
        self,
        _: Timestamp,
        _: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        Err(ContractError::unsupported_operation(
            "Configuration changes are not allowed in this state!",
        ))
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse>;
}

#[derive(Serialize, Deserialize)]
enum StateEnum {
    OpenIca(IcaConnector),
    Idle(Idle),
    BuyBack(
        StateLocalOut<
            BuyBack,
            PaymentGroup,
            SwapClient,
            ForwardToDexEntry,
            ForwardToDexEntryContinue,
        >,
    ),
}

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct State(StateEnum);

impl ConfigManagement for State {
    fn try_update_config(
        self,
        now: Timestamp,
        cadence_hours: CadenceHours,
    ) -> ContractResult<StateMachineResponse<Self>> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica
                .try_update_config(now, cadence_hours)
                .map(state_machine::from),
            StateEnum::Idle(idle) => idle
                .try_update_config(now, cadence_hours)
                .map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back
                .try_update_config(now, cadence_hours)
                .map(state_machine::from),
        }
    }

    fn try_query_config(&self) -> ContractResult<ConfigResponse> {
        match &self.0 {
            StateEnum::OpenIca(ica) => ica.try_query_config(),
            StateEnum::Idle(idle) => idle.try_query_config(),
            StateEnum::BuyBack(buy_back) => buy_back.try_query_config(),
        }
    }
}

impl State {
    pub fn new_state(config: Config, dex: ConnectionParams) -> IcaConnector {
        IcaConnector::new(OpenIca::new(config, dex))
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        STATE.load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        STATE.save(storage, self).map_err(Into::into)
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

impl
    From<
        StateLocalOut<
            BuyBack,
            ProfitCurrencies,
            SwapClient,
            ForwardToDexEntry,
            ForwardToDexEntryContinue,
        >,
    > for State
{
    fn from(
        value: StateLocalOut<
            BuyBack,
            ProfitCurrencies,
            SwapClient,
            ForwardToDexEntry,
            ForwardToDexEntryContinue,
        >,
    ) -> Self {
        Self(StateEnum::BuyBack(value))
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
            StateEnum::OpenIca(ica) => ica.on_open_ica(counterparty_version, deps, env),
            StateEnum::Idle(idle) => idle.on_open_ica(counterparty_version, deps, env),
            StateEnum::BuyBack(buy_back) => buy_back
                .on_open_ica(counterparty_version, deps, env)
                .map(state_machine::from),
        }
    }

    fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_response(data, deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_response(data, deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_response(data, deps, env).map_into(),
        }
    }

    fn on_error(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_error(deps, env),
            StateEnum::Idle(idle) => idle.on_error(deps, env),
            StateEnum::BuyBack(buy_back) => buy_back.on_error(deps, env).map(state_machine::from),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_timeout(deps, env).map(state_machine::from),
            StateEnum::Idle(idle) => idle.on_timeout(deps, env).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back.on_timeout(deps, env).map(state_machine::from),
        }
    }

    fn on_inner(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_inner(deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_inner(deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_inner(deps, env).map_into(),
        }
    }

    fn on_inner_continue(self, deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_inner_continue(deps, env).map(state_machine::from),
            StateEnum::Idle(idle) => idle.on_inner_continue(deps, env).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back
                .on_inner_continue(deps, env)
                .map(state_machine::from),
        }
    }

    fn reply(self, deps: &mut DepsMut<'_>, env: Env, msg: CwReply) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.reply(deps, env, msg).map(state_machine::from),
            StateEnum::Idle(idle) => idle.reply(deps, env, msg).map(state_machine::from),
            StateEnum::BuyBack(buy_back) => buy_back.reply(deps, env, msg).map(state_machine::from),
        }
    }

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenIca(ica) => ica.on_time_alarm(deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_time_alarm(deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => buy_back.on_time_alarm(deps, env).map_into(),
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            StateEnum::OpenIca(ica) => Display::fmt(&ica, f),
            StateEnum::Idle(idle) => Display::fmt(&idle, f),
            StateEnum::BuyBack(buy_back) => Display::fmt(&buy_back, f),
        }
    }
}
