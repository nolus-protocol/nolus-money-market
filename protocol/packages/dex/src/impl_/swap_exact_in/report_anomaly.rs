use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};

use crate::{AnomalyTreatment, SwapOutputTask, SwapTask as SwapTaskT, WithOutputTask};

pub struct ReportAnomalyCmd<SwapTask> {
    _spec: PhantomData<SwapTask>,
}

impl<SwapTask> Default for ReportAnomalyCmd<SwapTask> {
    fn default() -> Self {
        Self {
            _spec: Default::default(),
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
        task.on_anomaly()
    }
}
