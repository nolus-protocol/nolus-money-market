use platform::{message::Response as MessageResponse, state_machine};
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

use super::{DexState, DrainState, Task};

pub(super) trait ClosePositionTask<CalculatorT>
where
    CalculatorT: Calculator,
    Self: IntoRepayable + Sized,
    DexState<Self::Repayable, CalculatorT>: Into<State>,
    DrainState<Self::Repayable>: Into<State>,
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
        match dex::start_swap(Task::new(lease, self.into(), slippage_calc), env, querier) {
            dex::Result::Continue(cont) => cont
                .map(state_machine::from)
                .map(|swap: Response| {
                    Response::from(
                        curr_request_response.merge_with(swap.response),
                        swap.next_state,
                    )
                })
                .map_err(Into::into),
            // A position close always holds the asset to sell, so the first
            // swap leg is always scheduled - the start never finishes
            // synchronously the way a folded, nothing-to-swap task would.
            dex::Result::Finished(_finished) => {
                unreachable!("a position close always has the asset to sell")
            }
        }
    }
}
impl<CalculatorT, T> ClosePositionTask<CalculatorT> for T
where
    T: IntoRepayable,
    CalculatorT: Calculator,
    DexState<T::Repayable, CalculatorT>: Into<State>,
    DrainState<T::Repayable>: Into<State>,
{
}
