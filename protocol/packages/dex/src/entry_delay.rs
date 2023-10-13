use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::Result as DexResult, Contract, Handler, Result};
#[cfg(feature = "migration")]
use crate::{InspectSpec, MigrateSpec};

use super::{ica_connector::Enterable as EnterableT, Response};

#[derive(Serialize, Deserialize)]
pub struct EntryDelay<Enterable> {
    enterable: Enterable,
    time_alarms: TimeAlarmsRef,
}

impl<Enterable> EntryDelay<Enterable> {
    const RIGHT_AFTER_NOW: Duration = Duration::from_nanos(1);

    pub(super) fn new(enterable: Enterable, time_alarms: TimeAlarmsRef) -> Self {
        Self {
            enterable,
            time_alarms,
        }
    }

    pub(super) fn enter(&self, now: Timestamp) -> DexResult<Batch> {
        self.time_alarms
            .setup_alarm(now + Self::RIGHT_AFTER_NOW)
            .map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnumNew, Enterable> MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for EntryDelay<Enterable>
where
    Enterable: MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>,
{
    type Out = EntryDelay<Enterable::Out>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(self.enterable.migrate_spec(migrate_fn), self.time_alarms)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, Enterable> InspectSpec<SwapTask, R> for EntryDelay<Enterable>
where
    Enterable: InspectSpec<SwapTask, R>,
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        self.enterable.inspect_spec(inspect_fn)
    }
}

impl<Enterable> EnterableT for EntryDelay<Enterable> {
    fn enter(&self, now: Timestamp, _querier: &QuerierWrapper<'_>) -> DexResult<Batch> {
        Self::enter(self, now)
    }
}

impl<Enterable, R, SR> Handler for EntryDelay<Enterable>
where
    Enterable: EnterableT + Handler<Response = R, SwapResult = SR> + Into<R>,
{
    type Response = R;
    type SwapResult = SR;

    fn on_time_alarm(self, deps: Deps<'_>, env: Env) -> Result<Self> {
        self.enterable
            .enter(env.block.time, &deps.querier)
            .map(|batch| Response::<Self>::from(batch, self.enterable))
            .into()
    }
}

impl<Connectee> Contract for EntryDelay<Connectee>
where
    Connectee: Contract,
{
    type StateResponse = Connectee::StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        self.enterable.state(now, querier)
    }
}

impl<Connectee> Display for EntryDelay<Connectee>
where
    Connectee: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("EntryDelay({})", self.enterable))
    }
}
