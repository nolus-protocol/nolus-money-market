use finance::duration::Duration;
use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};
use serde::{Deserialize, Serialize};
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

    pub(super) fn new(enterable: Enterable, time_alarms: TimeAlarmsRef) -> Self {
        Self {
            enterable,
            time_alarms,
        }
    }
}

impl<Enterable> EnterableT for EntryDelay<Enterable> {
    fn enter(&self, now: Timestamp, _querier: &QuerierWrapper<'_>) -> DexResult<Batch> {
        self.time_alarms
            .clone()
            .setup_alarm(now + Self::RIGHT_AFTER_NOW)
            .map_err(Into::into)
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
