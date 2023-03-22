use cosmwasm_std::{QuerierWrapper, Timestamp};
use platform::batch::Batch;
use serde::{Deserialize, Serialize};

use crate::{
    api::{dex::ConnectionParams, StateResponse},
    contract::{
        dex::{Account, DexConnectable},
        state::State,
        Contract,
    },
    error::ContractResult,
};

use super::{
    ica_connector::{Enterable, IcaConnectee},
    ica_post_connector::Postpone,
};

#[derive(Serialize, Deserialize)]
pub struct InRecovery<S> {
    state: S,
}

impl<S> InRecovery<S> {
    pub(super) fn new(state: S) -> Self {
        Self { state }
    }
}

impl<S> IcaConnectee for InRecovery<S>
where
    S: Enterable + Into<State>,
{
    const PRECONNECTABLE: bool = false;
    type NextState = S;

    fn connected(self, _dex_account: Account) -> S {
        self.state
    }
}

impl<S> DexConnectable for InRecovery<S>
where
    S: DexConnectable,
{
    fn dex(&self) -> &ConnectionParams {
        self.state.dex()
    }
}

impl<S> Contract for InRecovery<S>
where
    S: Contract,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<StateResponse> {
        self.state.state(now, querier)
    }
}

impl<S> Postpone for InRecovery<S>
where
    S: Postpone,
{
    fn setup_alarm(&self, when: Timestamp, querier: &QuerierWrapper<'_>) -> ContractResult<Batch> {
        self.state.setup_alarm(when, querier)
    }
}
