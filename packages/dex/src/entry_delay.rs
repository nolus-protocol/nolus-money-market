use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};
use timealarms::stub::TimeAlarmsRef;

use crate::{error::Result as DexResult, Contract, Handler, Result};

use super::{ica_connector::Enterable as EnterableT, Response};

#[derive(Serialize, Deserialize)]
pub struct EntryDelay<Enterable> {
    enterable: Enterable,
    time_alarms: TimeAlarmsRef,
}

impl<Enterable> EntryDelay<Enterable> {
    const RIGHT_AFTER_NOW: Duration = Duration::from_nanos(1);

    #[cfg(feature = "migration")]
    pub fn new_migrate(enterable: Enterable, time_alarms: TimeAlarmsRef) -> Self {
        Self::new(enterable, time_alarms)
    }

    pub(super) fn new(enterable: Enterable, time_alarms: TimeAlarmsRef) -> Self {
        Self {
            enterable,
            time_alarms,
        }
    }

    #[cfg(feature = "migration")]
    pub fn enter_migrate(&self, now: Timestamp) -> DexResult<Batch> {
        self.enter(now)
    }

    pub(super) fn enter(&self, now: Timestamp) -> DexResult<Batch> {
        self.time_alarms
            .setup_alarm(now + Self::RIGHT_AFTER_NOW)
            .map_err(Into::into)
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
