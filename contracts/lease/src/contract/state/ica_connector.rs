use serde::{Deserialize, Serialize};

use platform::{
    batch::{Batch, Emit, Emitter},
    ica::HostAccount,
};
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};

use crate::{
    api::StateResponse,
    contract::{
        dex::{Account, DexConnectable},
        state::{self, ica_post_connector::PostConnector, Controller, Response},
        Contract,
    },
    error::ContractResult,
    event::Type,
};

use super::{ica_post_connector::Postpone, State};

pub(crate) trait Enterable {
    fn enter(&self, deps: Deps<'_>, _env: Env) -> ContractResult<Batch>;
}

/// Entity expecting to be connected to ICA
///
/// Due to the fact that at the time we get the acknowledgement the underlying channel
/// is not yet fully functional, we are not allowed to use it right away.
/// There are usecases that do not use it immediatelly so they are ok to go at
/// this "preconnection" state. The others should be called in a next block to the
/// one that delivers the acknowledgement. Usually that could be done with
/// a time alarm.
pub(crate) trait IcaConnectee {
    /// Designates if this entity is ready to process the ICA connection at
    /// its "preconnection" state.
    ///
    /// If true they do other non-ICS27 channel activities.
    /// If false they rely on a fully functional, and with an open underlying channel, ICA.
    const PRECONNECTABLE: bool;
    type NextState: Enterable + Into<State>;

    fn connected(self, ica_account: Account) -> Self::NextState;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct IcaConnector<const PRECONNECTABLE: bool, Connectee> {
    connectee: Connectee,
}

impl<const PRECONNECTABLE: bool, Connectee> IcaConnector<PRECONNECTABLE, Connectee>
where
    Connectee: IcaConnectee + DexConnectable,
{
    pub(super) fn new(connectee: Connectee) -> Self {
        Self { connectee }
    }

    pub(super) fn enter(&self) -> Batch {
        Account::register_request(self.connectee.dex())
    }

    fn build_account(&self, counterparty_version: String, env: &Env) -> ContractResult<Account> {
        let contract = env.contract.address.clone();
        Account::from_register_response(
            &counterparty_version,
            contract,
            self.connectee.dex().clone(),
        )
    }

    fn emit_ok(env: &Env, dex_account: HostAccount) -> Emitter {
        let contract = env.contract.address.clone();
        Emitter::of_type(Type::OpenIcaAccount)
            .emit("id", contract)
            .emit("dex_account", dex_account)
    }
}

impl<const PRECONNECTABLE: bool, Connectee> Enterable for IcaConnector<PRECONNECTABLE, Connectee>
where
    Connectee: IcaConnectee + DexConnectable,
{
    fn enter(&self, _deps: Deps<'_>, _env: Env) -> ContractResult<Batch> {
        Ok(self.enter())
    }
}

impl<Connectee> Controller for IcaConnector<true, Connectee>
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
        let ica = self.build_account(counterparty_version, &env)?;

        let emitter = Self::emit_ok(&env, ica.ica_account().clone());

        let next_state = self.connectee.connected(ica);

        next_state
            .enter(deps, env)
            .map(|batch| Response::from(batch.into_response(emitter), next_state))
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::OpenIcaAccount, deps, env)
    }
}

impl<Connectee> Controller for IcaConnector<false, Connectee>
where
    Self: Into<State>,
    PostConnector<Connectee>: Into<State>,
    Connectee: IcaConnectee + DexConnectable + Postpone,
{
    fn on_open_ica(
        self,
        counterparty_version: String,
        deps: Deps<'_>,
        env: Env,
    ) -> ContractResult<Response> {
        let ica = self.build_account(counterparty_version, &env)?;

        let emitter = Self::emit_ok(&env, ica.ica_account().clone());

        let next_state = PostConnector::new(self.connectee, ica);

        next_state
            .enter(env.block.time, &deps.querier)
            .map(|batch| Response::from(batch.into_response(emitter), next_state))
            .map_err(Into::into)
    }

    fn on_timeout(self, deps: Deps<'_>, env: Env) -> ContractResult<Response> {
        state::on_timeout_retry(self, Type::OpenIcaAccount, deps, env)
    }
}

impl<const PRECONNECTABLE: bool, Connectee> Contract for IcaConnector<PRECONNECTABLE, Connectee>
where
    Connectee: Contract,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.connectee.state(now, querier)
    }
}
