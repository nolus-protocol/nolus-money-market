use serde::{Deserialize, Serialize};

use crate::{
    api::{dex::ConnectionParams, StateResponse},
    contract::{
        dex::{Account, DexConnectable},
        state::State,
    },
};

use super::{ica_connector::IcaConnectee, Controller};

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
    S: Controller + Into<State>,
{
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

impl<S> From<InRecovery<S>> for StateResponse {
    fn from(_value: InRecovery<S>) -> Self {
        todo!("use a fn from Controller")
    }
}
