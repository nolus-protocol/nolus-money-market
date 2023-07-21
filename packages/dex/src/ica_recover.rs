use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use platform::batch::Batch;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{
    account::Account, connectable::DexConnectable, connection::ConnectionParams,
    entry_delay::EntryDelay, error::Result as DexResult, Contract, TimeAlarm,
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

impl<S, SEnum> Display for InRecovery<S, SEnum>
where
    S: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("InRecovery({})", self.state))
    }
}

impl<S, SEnum> TimeAlarm for InRecovery<S, SEnum> {
    fn setup_alarm(&self, forr: Timestamp) -> DexResult<Batch> {
        self.time_alarms.setup_alarm(forr).map_err(Into::into)
    }
}
