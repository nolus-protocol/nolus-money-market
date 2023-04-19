use entry_delay::EntryDelay;
use sdk::cosmwasm_std::{QuerierWrapper, Timestamp};

mod account;
mod coin_index;
mod connectable;
mod connection;
mod entry_delay;
mod error;
mod filter;
mod ica_connector;
mod ica_recover;
mod never;
mod out_local;
mod out_remote;
mod response;
mod swap_coins;
mod swap_exact_in;
mod swap_task;
mod timeout;
mod transfer_in;
mod transfer_in_finish;
mod transfer_in_init;
mod transfer_out;
mod trx;

use crate::SwapTask as SwapTaskT;
pub use {
    account::Account,
    connectable::DexConnectable,
    connection::{ConnectionParams, Ics20Channel},
    error::Error,
    ica_connector::{
        Enterable, IcaConnectee, IcaConnector, ICS27_MESSAGE_ENTERING_NEXT_STATE,
        NO_ICS27_MESSAGE_ENTERING_NEXT_STATE,
    },
    ica_recover::InRecovery,
    out_local::{start_local_local, StartLocalLocalState, State as StateLocalOut},
    out_remote::{start as start_local_remote, StartLocalRemoteState, State as StateRemoteOut},
    response::{ContinueResult, Handler, Response, Result},
    swap_coins::{on_coin, on_coins},
    swap_exact_in::SwapExactIn,
    swap_task::{CoinVisitor, CoinsNb, IterNext, IterState, SwapTask},
    transfer_in_finish::TransferInFinish,
    transfer_in_init::TransferInInit,
    transfer_out::TransferOut,
};

type SwapExactInRecoverIca<SwapTask, SEnum> =
    IcaConnector<InRecovery<SwapExactIn<SwapTask, SEnum>, SEnum>, <SwapTask as SwapTaskT>::Result>;

type SwapExactInPostRecoverIca<SwapTask, SEnum> = EntryDelay<SwapExactIn<SwapTask, SEnum>>;

type TransferInInitRecoverIca<SwapTask, SEnum> =
    IcaConnector<InRecovery<TransferInInit<SwapTask>, SEnum>, <SwapTask as SwapTaskT>::Result>;

type TransferInInitPostRecoverIca<SwapTask> = EntryDelay<TransferInInit<SwapTask>>;

/// Contract during DEX
pub trait Contract
where
    Self: Sized,
{
    type StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse;
}

pub struct TransferOutState {}
pub struct SwapState {}
pub struct TransferInInitState {}
pub struct TransferInFinishState {}

/// Contract in a swap state
///
/// The states are `TransferOutState`, `SwapState`, and `TransferInState`
pub trait ContractInSwap<State, StateResponse>
where
    Self: Sized,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> StateResponse;
}
