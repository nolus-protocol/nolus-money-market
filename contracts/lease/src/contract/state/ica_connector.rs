use cosmwasm_std::{QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};

use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::cosmwasm_std::{Addr, Deps, Env};

use crate::{
    api::StateResponse,
    contract::{
        dex::{Account, DexConnectable},
        state::{self, Controller, Response},
        Contract,
    },
    error::ContractResult,
    event::Type,
};

use super::State;

pub(crate) trait Enterable {
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<Batch>;
}

pub(crate) trait IcaConnectee {
    type NextState: Enterable + Into<State>;

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

    pub(super) fn enter(&self) -> Batch {
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

impl<Connectee> Enterable for IcaConnector<Connectee>
where
    Connectee: IcaConnectee + DexConnectable,
{
    fn enter(&self, _deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        Ok(self.enter())
    }
}

impl<Connectee> Controller for IcaConnector<Connectee>
where
    Self: Into<State>,
    Connectee: IcaConnectee + DexConnectable,
{
    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        self.on_response(counterparty_version, deps, env)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::OpenIcaAccount, deps, env)
    }
}

impl<Connectee> Contract for IcaConnector<Connectee>
where
    Connectee: Contract,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.connectee.state(now, querier)
    }
}
