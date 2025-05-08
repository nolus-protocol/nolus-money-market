use dex::{AnomalyHandler, Enterable, SlippageCalculator};
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{
    api::{
        LeaseAssetCurrencies, LeaseCoin,
        query::opened::{OngoingTrx, PositionCloseTrx},
    },
    contract::{
        Lease,
        state::{Response, State, SwapClient, resp_delivery::ForwardToDexEntry},
    },
    error::ContractResult,
    event::Type,
    finance::LpnCurrency,
};

use self::sell_asset::SellAsset;
pub(crate) use anomaly::SlippageAnomaly;

use super::payment::Repayable;

mod anomaly;
pub mod customer_close;
pub mod liquidation;
pub mod sell_asset;

pub(crate) trait Closable {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin;
    fn transaction(&self, lease: &Lease, in_progress: PositionCloseTrx) -> OngoingTrx;
    fn event_type(&self) -> Type;
}

type Task<RepayableT, CalculatorT> = SellAsset<RepayableT, CalculatorT>;
type DexState<Repayable, CalculatorT> =
    dex::StateLocalOut<Task<Repayable, CalculatorT>, SwapClient, ForwardToDexEntry>;

/// Aim to simplify trait boundaries within this module and underneat
pub(crate) trait Calculator
where
    Self: SlippageCalculator<LeaseAssetCurrencies, OutC = LpnCurrency>,
{
}

trait IntoRepayable
where
    Self::Repayable: Closable + Repayable,
{
    type Repayable;

    fn into(self) -> Self::Repayable;
}

trait ClosePositionTask<CalculatorT>
where
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
    CalculatorT: Calculator,
    Task<Self::Repayable, CalculatorT>: AnomalyHandler<Task<Self::Repayable, CalculatorT>>,
    DexState<T::Repayable, CalculatorT>: Into<State>,
{
}
