use finance::duration::Duration;
use finance::instant::Instant;
use platform::batch::Batch;
use timealarms::stub::TimeAlarmsRef;

use crate::error::Result;

const POLLING_INTERVAL: Duration = Duration::from_secs(5);

pub(super) fn setup_alarm(time_alarms: &TimeAlarmsRef, now: Instant) -> Result<Batch> {
    time_alarms
        .setup_alarm(now + POLLING_INTERVAL)
        .map_err(Into::into)
}
