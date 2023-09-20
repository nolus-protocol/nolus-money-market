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

fn start_impl<Spec, RepayableT>(
    lease: Lease,
    spec: Spec,
    curr_request_response: MessageResponse,
    env: &Env,
    querier: &QuerierWrapper<'_>,
) -> ContractResult<Response>
where
    Spec: Into<RepayableT>,
    RepayableT: Closable + Repayable,
    DexState<RepayableT>: Into<State>,
{
    let start_state = dex::start_remote_local(Task::new(lease, spec.into()));
    start_state
        .enter(env.block.time, querier)
        .map(|swap_msg| curr_request_response.merge_with(swap_msg))
        .map(|start| Response::from(start, DexState::<RepayableT>::from(start_state)))
        .map_err(Into::into)
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
