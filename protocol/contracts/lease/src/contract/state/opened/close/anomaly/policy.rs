use currency::{CurrencyDTO, Group, MemberOf};
use dex::{AnomalyPolicy, AnomalyTreatment, SwapTask};
use finance::{coin::CoinDTO, percent::Percent};
use platform::state_machine::Response;

use crate::contract::state::{
    State,
    opened::{
        close::{Closable, sell_asset::SellAsset},
        event,
        payment::Repayable,
    },
};

use super::SlippageAnomaly;

pub struct MaxSlippage<G>
where
    G: Group,
{
    max_slippage: Percent,
    out_currency: CurrencyDTO<G>,
}

impl<G> MaxSlippage<G>
where
    G: Group,
{
    // pub fn on_task<SwapTaskT>(_task: &SwapTaskT) -> Self
    // where
    //     SwapTaskT: SwapTask<OutG = G>,
    // {
    // Self {
    //     max_slippage,
    //     out_currency: task.out_currency(),
    // };
    // }
}

impl<RepayableT> AnomalyPolicy<SellAsset<RepayableT>>
    for MaxSlippage<<SellAsset<RepayableT> as SwapTask>::OutG>
where
    RepayableT: Closable + Repayable,
    SlippageAnomaly<RepayableT>: Into<State>,
{
    fn min_output<InG>(
        &self,
        _input: &CoinDTO<InG>,
    ) -> CoinDTO<<SellAsset<RepayableT> as SwapTask>::OutG>
    where
        InG: Group + MemberOf<<SellAsset<RepayableT> as SwapTask>::InG>,
    {
        let _ = self.out_currency; //avoid the unused member warning
        todo!("TODO use oracle_platform::convert::{{from|to}}_quote(..)")
    }

    fn on_anomaly(&self, task: SellAsset<RepayableT>) -> AnomalyTreatment<SellAsset<RepayableT>>
    where
        Self: Sized,
    {
        let (lease, repayable) = task.drop();
        let emitter = event::emit_slippage_anomaly(&lease.lease, self.max_slippage);
        let next_state = SlippageAnomaly::new(lease, repayable);
        AnomalyTreatment::Exit(Ok(Response::from(emitter, next_state)))
    }
}
