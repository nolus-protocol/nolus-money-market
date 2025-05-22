use dex::{AnomalyHandler, Enterable};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    contract::{
        Lease,
        state::{
            Response, State,
            opened::close::{Calculator, IntoRepayable},
        },
    },
    error::ContractResult,
};

use super::{DexState, Task};

pub(super) trait ClosePositionTask<CalculatorT>
where
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
    CalculatorT: Calculator,
    Self: IntoRepayable + Sized,
    Task<Self::Repayable, CalculatorT>: AnomalyHandler<Task<Self::Repayable, CalculatorT>>,
    DexState<Self::Repayable, CalculatorT>: Into<State>,
{
    fn start(
        self,
        lease: Lease,
        curr_request_response: MessageResponse,
        slippage_calc: CalculatorT,
        env: &Env,
        querier: QuerierWrapper<'_>,
    ) -> ContractResult<Response>
where {
        let start_state = dex::start_remote_local(Task::new(lease, self.into(), slippage_calc));
        start_state
            .enter(env.block.time, querier)
            .map(|swap_msg| curr_request_response.merge_with(swap_msg))
            .map(|start| {
                Response::from(
                    start,
                    DexState::<Self::Repayable, CalculatorT>::from(start_state),
                )
            })
            .map_err(Into::into)
    }
}
impl<CalculatorT, T> ClosePositionTask<CalculatorT> for T
where
    T: IntoRepayable,
    //TODO remove past the migration from v0.8.7
    CalculatorT: Default,
    CalculatorT: Calculator,
    Task<Self::Repayable, CalculatorT>: AnomalyHandler<Task<Self::Repayable, CalculatorT>>,
    DexState<T::Repayable, CalculatorT>: Into<State>,
{
}
