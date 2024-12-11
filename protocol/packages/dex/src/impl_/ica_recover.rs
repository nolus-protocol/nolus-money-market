use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use finance::duration::Duration;
use platform::batch::Batch;
use serde::{Deserialize, Serialize};

use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::Result as DexResult, ConnectionParams};
#[cfg(feature = "migration")]
use crate::{InspectSpec, MigrateSpec};

use super::{Account, Contract, DexConnectable, Enterable, IcaConnectee, TimeAlarm};

#[derive(Serialize, Deserialize)]
#[serde(bound(serialize = "S: Serialize", deserialize = "S: Deserialize<'de>"))]
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

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnumNew, S, SEnum> MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for InRecovery<S, SEnum>
where
    S: MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>,
    S::Out: Into<SEnumNew>,
{
    type Out = InRecovery<S::Out, SEnumNew>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(self.state.migrate_spec(migrate_fn), self.time_alarms)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, S, SEnum> InspectSpec<SwapTask, R> for InRecovery<S, SEnum>
where
    S: InspectSpec<SwapTask, R>,
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        self.state.inspect_spec(inspect_fn)
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
    S: Enterable + Into<SEnum>,
{
    type State = SEnum;
    type NextState = S;

    fn connected(self, _dex_account: Account) -> Self::NextState {
        self.state
    }
}

impl<S, SEnum> Contract for InRecovery<S, SEnum>
where
    S: Contract,
{
    type StateResponse = S::StateResponse;

    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.state.state(now, due_projection, querier)
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
