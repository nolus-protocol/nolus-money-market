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

#[cfg(test)]
mod test {
    use currency::test::{SuperGroup, SuperGroupTestC1};
    use finance::coin::Coin;
    use sdk::cosmwasm_std::{Env, MessageInfo, QuerierWrapper};
    use timealarms::stub::TimeAlarmsRef;

    use crate::{
        Account, AnomalyCause, AnomalyTreatment, SwapCoins, SwapOutputTask, SwapTask,
        WithOutputTask, error::Result as DexResult, slippage::WithCalculator,
    };

    use super::ReportAnomalyCmd;

    // Whatever the workflow hands the leg must be what the leg is asked about. A
    // command that dropped the cause, or defaulted it, would still type-check and
    // would silently route every anomaly alike.
    #[test]
    fn the_command_delivers_the_cause_to_the_leg() {
        assert!(matches!(
            report(AnomalyCause::MinOutputNotFulfilled),
            AnomalyTreatment::Exit(AnomalyCause::MinOutputNotFulfilled),
        ));
        assert!(matches!(
            report(AnomalyCause::Other),
            AnomalyTreatment::Exit(AnomalyCause::Other),
        ));
    }

    fn report(cause: AnomalyCause) -> AnomalyTreatment<Spec> {
        Spec.into_output_task(ReportAnomalyCmd::new(cause))
    }

    struct Spec;

    // `Result = AnomalyCause` is what makes the cause observable: the leg below
    // exits with the very value it was asked about, so the assertion reads it out
    // of the treatment instead of through interior mutability.
    impl SwapTask for Spec {
        type InG = SuperGroup;
        type OutG = SuperGroup;
        type Label = &'static str;
        type StateResponse = ();
        type Result = AnomalyCause;

        fn label(&self) -> Self::Label {
            "report-anomaly-spec"
        }

        fn dex_account(&self) -> &Account {
            unimplemented!("the anomaly report never reaches the account")
        }

        fn time_alarm(&self) -> &TimeAlarmsRef {
            unimplemented!("the anomaly report schedules nothing")
        }

        fn authz_remote_callback(
            &self,
            _querier: QuerierWrapper<'_>,
            _info: &MessageInfo,
        ) -> DexResult<()> {
            unimplemented!("the caller authorises before reporting")
        }

        fn coins(&self) -> SwapCoins<Self::InG> {
            unimplemented!("the anomaly report re-reads no input")
        }

        fn with_slippage_calc<WithCalc>(&self, _with_calc: WithCalc) -> WithCalc::Output
        where
            WithCalc: WithCalculator<Self>,
        {
            unimplemented!("only re-entering the swap quotes a floor")
        }

        fn into_output_task<Cmd>(self, cmd: Cmd) -> Cmd::Output
        where
            Cmd: WithOutputTask<Self>,
        {
            cmd.on(OutputTask(self))
        }
    }

    struct OutputTask(Spec);

    impl SwapOutputTask<Spec> for OutputTask {
        type OutC = SuperGroupTestC1;

        fn as_spec(&self) -> &Spec {
            &self.0
        }

        fn into_spec(self) -> Spec {
            self.0
        }

        fn on_anomaly(self, cause: AnomalyCause) -> AnomalyTreatment<Spec> {
            AnomalyTreatment::Exit(cause)
        }

        fn finish(
            self,
            _amount_out: Coin<Self::OutC>,
            _env: &Env,
            _querier: QuerierWrapper<'_>,
        ) -> <Spec as SwapTask>::Result {
            unimplemented!("the anomaly path never finishes the swap")
        }
    }
}
