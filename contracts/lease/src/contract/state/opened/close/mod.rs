use dex::Enterable;
use platform::message::Response as MessageResponse;
use sdk::cosmwasm_std::{Env, QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        LeaseCoin, StateResponse,
    },
    contract::{
        state::{
            resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
            Response, State,
        },
        Lease,
    },
    error::ContractResult,
    event::Type,
};

use self::sell_asset::SellAsset;

use super::payment::Repayable;

pub mod customer_close;
pub mod liquidation;
mod sell_asset;

pub(crate) trait Closable {
    fn amount<'a>(&'a self, lease: &'a Lease) -> &'a LeaseCoin;
    fn event_type(&self) -> Type;
}

type Task<RepayableT> = SellAsset<RepayableT>;
type DexState<Repayable> =
    dex::StateLocalOut<Task<Repayable>, ForwardToDexEntry, ForwardToDexEntryContinue>;

trait IntoRepayable
where
    Self::Repayable: Closable + Repayable,
{
    type Repayable;

    fn into(self) -> Self::Repayable;
}

trait ClosePositionTask
where
    Self: IntoRepayable + Sized,
    DexState<Self::Repayable>: Into<State>,
{
    fn start(
        self,
        lease: Lease,
        curr_request_response: MessageResponse,
        env: &Env,
        querier: &QuerierWrapper<'_>,
    ) -> ContractResult<Response>
where {
        let start_state = dex::start_remote_local(Task::new(lease, self.into()));
        start_state
            .enter(env.block.time, querier)
            .map(|swap_msg| curr_request_response.merge_with(swap_msg))
            .map(|start| Response::from(start, DexState::<Self::Repayable>::from(start_state)))
            .map_err(Into::into)
    }
}
impl<T> ClosePositionTask for T
where
    T: IntoRepayable,
    DexState<T::Repayable>: Into<State>,
{
}

fn query<RepayableT>(
    lease: Lease,
    repayable: RepayableT,
    in_progress: LiquidateTrx,
    now: Timestamp,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<StateResponse>
where
    RepayableT: Closable,
{
    let in_progress = OngoingTrx::Liquidation {
        liquidation: repayable.amount(&lease).clone(),
        in_progress,
    };

    super::lease_state(lease, Some(in_progress), now, querier)
}
