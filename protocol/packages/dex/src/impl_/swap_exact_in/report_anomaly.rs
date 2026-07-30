use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};

use crate::{
    AnomalyCause, AnomalyTreatment, SwapOutputTask, SwapTask as SwapTaskT, WithOutputTask,
};

// No `Default`: the cause is the whole point of this command, so a causeless
// report must not be constructible.
pub struct ReportAnomalyCmd<SwapTask> {
    cause: AnomalyCause,
    _spec: PhantomData<SwapTask>,
}

impl<SwapTask> ReportAnomalyCmd<SwapTask> {
    pub const fn new(cause: AnomalyCause) -> Self {
        Self {
            cause,
            _spec: PhantomData,
        }
    }
}

impl<SwapTask> WithOutputTask<SwapTask> for ReportAnomalyCmd<SwapTask>
where
    SwapTask: SwapTaskT,
{
    type Output = AnomalyTreatment<SwapTask>;

    fn on<OutC, OutputTaskT>(self, task: OutputTaskT) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG>,
        OutputTaskT: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        task.on_anomaly(self.cause)
    }
}
