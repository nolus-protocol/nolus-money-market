use dex::StartRemoteLocalState;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

use crate::{
    api::{
        opened::{LiquidateTrx, OngoingTrx},
        StateResponse,
    },
    contract::{
        cmd::Closable,
        state::resp_delivery::{ForwardToDexEntry, ForwardToDexEntryContinue},
        Lease,
    },
    error::ContractResult,
};

use self::sell_asset::SellAsset;

pub(super) use liquidation::start as start_liquidation;

use super::payment::Repayable;

// mod customer_close;
pub mod liquidation;
mod sell_asset;

type Task<RepayableT> = SellAsset<RepayableT>;
type StartState<Repayable> =
    StartRemoteLocalState<Task<Repayable>, ForwardToDexEntry, ForwardToDexEntryContinue>;
type DexState<Repayable> =
    dex::StateLocalOut<Task<Repayable>, ForwardToDexEntry, ForwardToDexEntryContinue>;

fn start<RepayableT>(lease: Lease, repayable: RepayableT) -> StartState<RepayableT>
where
    RepayableT: Closable + Repayable,
{
    dex::start_remote_local(Task::new(lease, repayable))
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
