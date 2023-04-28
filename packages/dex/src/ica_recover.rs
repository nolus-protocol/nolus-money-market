use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    account::Account, connectable::DexConnectable, connection::ConnectionParams,
    entry_delay::EntryDelay, Contract,
};

use super::ica_connector::{Enterable, IcaConnectee};

#[derive(Serialize, Deserialize)]
pub struct InRecovery<S, SEnum> {
    state: S,
    time_alarms: TimeAlarmsRef,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<S, SEnum> InRecovery<S, SEnum> {
    pub(super) fn new(state: S, time_alarms: TimeAlarmsRef) -> Self {
        Self {
            state,
            time_alarms,
            _state_enum: PhantomData,
        }
    }
}

impl<State, StateEnum> Borrow<State> for InRecovery<State, StateEnum> {
    fn borrow(&self) -> &State {
        &self.state
    }
}

impl<State, StateEnum> BorrowMut<State> for InRecovery<State, StateEnum> {
    fn borrow_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl<S, SEnum> DexConnectable for InRecovery<S, SEnum>
where
    S: DexConnectable,
{
    fn dex(&self) -> &ConnectionParams {
        self.state.dex()
    }
}

impl<S, SEnum> IcaConnectee for InRecovery<S, SEnum>
where
    S: Enterable,
    EntryDelay<S>: Into<SEnum>,
{
    type State = SEnum;
    type NextState = EntryDelay<S>;

    fn connected(self, _dex_account: Account) -> Self::NextState {
        EntryDelay::new(self.state, self.time_alarms)
    }
}

impl<S, SEnum> Contract for InRecovery<S, SEnum>
where
    S: Contract,
{
    type StateResponse = S::StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        self.state.state(now, querier)
    }
}
