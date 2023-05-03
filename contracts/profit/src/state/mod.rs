use serde::{Deserialize, Serialize};

use dex::{
    ContinueResult, Handler, Ics20Channel, Response as DexResponse, Result as DexResult,
    StateLocalOut,
};
use oracle::stub::OracleRef;
use platform::state_machine;
use sdk::{
    cosmwasm_std::{Addr, Binary, Deps, Env, QuerierWrapper, Storage},
    cw_storage_plus::Item,
};
use timealarms::stub::TimeAlarmsRef;

use crate::{result::ContractResult, ContractError};

pub(crate) use self::config::Config;
use self::{
    buy_back::BuyBack, idle::Idle, open_ica::OpenIca, open_transfer_channel::OpenTransferChannel,
};

mod buy_back;
mod config;
mod idle;
mod open_ica;
mod open_transfer_channel;

const STATE: Item<'static, State> = Item::new("contract_state");

type IcaConnector = dex::IcaConnector<OpenIca, DexResponse<Idle>>;

trait UpdateConfig
where
    Self: Sized,
{
    fn update_config(self, cadence_hours: u16) -> Self;
}

pub(crate) trait ProfitMessageHandler
where
    Self: Handler,
{
    fn confirm_open(
        self,
        deps: Deps<'_>,
        env: Env,
        _channel: Ics20Channel,
        counterparty_version: String,
    ) -> ContinueResult<Self> {
        self.on_open_ica(counterparty_version, deps, env)
    }
}

#[derive(Serialize, Deserialize)]
#[repr(transparent)]
#[serde(transparent)]
pub(crate) struct State(StateEnum);

#[derive(Serialize, Deserialize)]
enum StateEnum {
    OpenTransferChannel(OpenTransferChannel),
    OpenIca(IcaConnector),
    Idle(Idle),
    BuyBack(StateLocalOut<BuyBack>),
}

impl State {
    pub fn new(
        querier: &QuerierWrapper<'_>,
        config: Config,
        connection_id: String,
        oracle_addr: Addr,
        time_alarms_addr: Addr,
    ) -> ContractResult<Self> {
        Ok(Self(StateEnum::OpenTransferChannel(
            OpenTransferChannel::new(
                config,
                connection_id,
                OracleRef::try_from(oracle_addr, querier)?,
                TimeAlarmsRef::new(time_alarms_addr, querier)?,
            ),
        )))
    }

    pub fn load(storage: &dyn Storage) -> ContractResult<Self> {
        STATE.load(storage).map_err(Into::into)
    }

    pub fn store(&self, storage: &mut dyn Storage) -> ContractResult<()> {
        STATE.save(storage, self).map_err(Into::into)
    }

    pub fn try_update_config(self, cadence_hours: u16) -> ContractResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => Ok(Self(StateEnum::OpenTransferChannel(
                transfer.update_config(cadence_hours),
            ))),
            StateEnum::OpenIca(_) => Err(ContractError::unsupported_operation(
                "Configuration changes are not allowed during ICA opening process.",
            )),
            StateEnum::Idle(idle) => Ok(Self(StateEnum::Idle(idle.update_config(cadence_hours)))),
            StateEnum::BuyBack(_) => Err(ContractError::unsupported_operation(
                "Configuration changes are not allowed during buy-back.",
            )),
        }
    }

    pub fn config(&self) -> ContractResult<&Config> {
        match &self.0 {
            StateEnum::OpenTransferChannel(transfer) => Ok(transfer.config()),
            StateEnum::OpenIca(_) => Err(ContractError::unsupported_operation(
                "Querying configuration is not allowed during ICA opening process.",
            )),
            StateEnum::Idle(idle) => Ok(idle.config()),
            StateEnum::BuyBack(_) => Err(ContractError::unsupported_operation(
                "Querying configuration is not allowed during buy-back.",
            )),
        }
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

impl From<StateLocalOut<BuyBack>> for State {
    fn from(value: StateLocalOut<BuyBack>) -> Self {
        Self(StateEnum::BuyBack(value))
    }
}

impl ProfitMessageHandler for State {
    fn confirm_open(
        self,
        deps: Deps<'_>,
        env: Env,
        channel: Ics20Channel,
        counterparty_version: String,
    ) -> ContinueResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.confirm_open(deps, env, channel, counterparty_version)
            }
            StateEnum::OpenIca(ica) => ica.confirm_open(deps, env, channel, counterparty_version),
            StateEnum::Idle(idle) => idle.confirm_open(deps, env, channel, counterparty_version),
            StateEnum::BuyBack(buy_back) => buy_back
                .confirm_open(deps, env, channel, counterparty_version)
                .map(state_machine::from),
        }
    }
}

impl Handler for State {
    type Response = State;
    type SwapResult = DexResponse<State>;

    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContinueResult<Self> {
        if let StateEnum::OpenIca(ica) = self.0 {
            ica.on_open_ica(counterparty_version, deps, env)
        } else {
            unimplemented!()
        }
    }

    fn on_response(self, data: Binary, deps: Deps<'_>, env: Env) -> DexResult<Self> {
        match self.0 {
            StateEnum::OpenTransferChannel(transfer) => {
                transfer.on_response(data, deps, env).map_into()
            }
            StateEnum::OpenIca(ica) => ica.on_response(data, deps, env).map_into(),
            StateEnum::Idle(idle) => idle.on_response(data, deps, env).map_into(),
            StateEnum::BuyBack(buy_back) => match buy_back.on_response(data, deps, env) {
                DexResult::Continue(result) => DexResult::Continue(result.map(state_machine::from)),
                DexResult::Finished(result) => DexResult::Continue(result),
            },
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
            StateEnum::BuyBack(buy_back) => match buy_back.on_time_alarm(deps, env) {
                DexResult::Continue(result) => DexResult::Continue(result.map(state_machine::from)),
                DexResult::Finished(result) => result.into(),
            },
        }
    }
}
