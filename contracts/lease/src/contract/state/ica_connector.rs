use serde::{Deserialize, Serialize};

use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::{
    cosmwasm_std::{Addr, Deps, DepsMut, Env},
    neutron_sdk::sudo::msg::SudoMsg,
};

use crate::{
    api::{StateQuery, StateResponse},
    contract::{
        dex::{Account, DexConnectable},
        state::{self, Controller, Response},
    },
    error::ContractResult,
    event::Type,
};

use super::State;

pub(crate) trait IcaConnectee
where
    Self: Into<StateResponse>,
{
    type NextState: Controller + Into<State>;

    fn connected(self, ica_account: Account) -> Self::NextState;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct IcaConnector<Connectee> {
    connectee: Connectee,
}

impl<Connectee> IcaConnector<Connectee>
where
    Connectee: IcaConnectee + DexConnectable,
{
    pub(super) fn new(connectee: Connectee) -> Self {
        Self { connectee }
    }

    fn enter_state(&self) -> Batch {
        Account::register_request(self.connectee.dex())
    }

    fn on_response(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        let contract = &env.contract.address;
        let ica_account = Account::from_register_response(
            &counterparty_version,
            contract.clone(),
            self.connectee.dex().clone(),
        )?;

        let emitter = Self::emit_ok(contract.clone(), ica_account.ica_account().clone());
        let next_state = self.connectee.connected(ica_account);
        let batch = next_state.enter(deps, env)?;
        Ok(Response::from(batch.into_response(emitter), next_state))
    }

    fn emit_ok(contract: Addr, dex_account: HostAccount) -> Emitter {
        Emitter::of_type(Type::OpenIcaAccount)
            .emit("id", contract)
            .emit("dex_account", dex_account)
    }
}

impl<Connectee> Controller for IcaConnector<Connectee>
where
    Self: Into<State>,
    Connectee: IcaConnectee + DexConnectable,
{
    fn enter(&self, _deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        Ok(self.enter_state())
    }

    fn sudo(self, deps: &mut DepsMut<'_>, env: Env, msg: SudoMsg) -> ContractResult<Response> {
        match msg {
            SudoMsg::OpenAck {
                port_id: _,
                channel_id: _,
                counterparty_channel_id: _,
                counterparty_version,
            } => self.on_response(counterparty_version, deps.as_ref(), env),
            SudoMsg::Timeout { request: _ } => self.on_timeout(deps.as_ref(), env),
            SudoMsg::Error {
                request: _,
                details: _,
            } => todo!(),
            _ => unreachable!(),
        }
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::OpenIcaAccount, deps, env)
    }

    fn query(self, _deps: Deps<'_>, _env: Env, _msg: StateQuery) -> ContractResult<StateResponse> {
        Ok(self.connectee.into())
    }
}
