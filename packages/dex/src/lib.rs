use platform::batch::Batch;
use resp_delivery::ResponseDelivery;
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp};
use serde::ser::Serialize;

pub use crate::{
    account::Account,
    connectable::DexConnectable,
    connection::{ConnectionParams, Ics20Channel},
    error::{Error, Result as DexResult},
    ica_connector::{
        Enterable, IcaConnectee, IcaConnector, ICS27_MESSAGE_ENTERING_NEXT_STATE,
        NO_ICS27_MESSAGE_ENTERING_NEXT_STATE,
    },
    ica_recover::InRecovery,
    out_local::{
        start_local_local, start_remote_local, StartLocalLocalState, StartRemoteLocalState,
        StartTransferInState, State as StateLocalOut,
    },
    out_remote::{start as start_local_remote, StartLocalRemoteState, State as StateRemoteOut},
    response::{ContinueResult, Handler, Response, Result},
    swap_coins::{on_coin, on_coins},
    swap_exact_in::SwapExactIn,
    swap_task::{CoinVisitor, CoinsNb, IterNext, IterState, SwapTask},
    transfer_in_finish::TransferInFinish,
    transfer_in_init::TransferInInit,
    transfer_out::TransferOut,
};
use crate::{entry_delay::EntryDelay, SwapTask as SwapTaskT};

mod account;
mod coin_index;
mod connectable;
mod connection;
mod entry_delay;
mod error;
mod filter;
mod ica_connector;
mod ica_recover;
mod out_local;
mod out_remote;
mod resp_delivery;
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

pub type TransferOutRespDelivery<SwapTask, SEnum, ForwardToInnerMsg> =
    ResponseDelivery<TransferOut<SwapTask, SEnum>, ForwardToInnerMsg>;

pub type SwapExactInRespDelivery<SwapTask, SEnum, ForwardToInnerMsg> =
    ResponseDelivery<SwapExactIn<SwapTask, SEnum>, ForwardToInnerMsg>;

pub type SwapExactInPreRecoverIca<SwapTask, SEnum> =
    EntryDelay<SwapExactInRecoverIca<SwapTask, SEnum>>;

pub type SwapExactInRecoverIca<SwapTask, SEnum> =
    IcaConnector<InRecovery<SwapExactIn<SwapTask, SEnum>, SEnum>, <SwapTask as SwapTaskT>::Result>;

pub type SwapExactInPostRecoverIca<SwapTask, SEnum> = EntryDelay<SwapExactIn<SwapTask, SEnum>>;

pub type TransferInInitRespDelivery<SwapTask, SEnum, ForwardToInnerMsg> =
    ResponseDelivery<TransferInInit<SwapTask, SEnum>, ForwardToInnerMsg>;

pub type TransferInInitPreRecoverIca<SwapTask, SEnum> =
    EntryDelay<TransferInInitRecoverIca<SwapTask, SEnum>>;

pub type TransferInInitRecoverIca<SwapTask, SEnum> = IcaConnector<
    InRecovery<TransferInInit<SwapTask, SEnum>, SEnum>,
    <SwapTask as SwapTaskT>::Result,
>;

pub type TransferInInitPostRecoverIca<SwapTask, SEnum> =
    EntryDelay<TransferInInit<SwapTask, SEnum>>;

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
/// The states are `TransferOutState`, `SwapState`, `TransferInInitState`, and `TransferInFinishState`
pub trait ContractInSwap<State, StateResponse>
where
    Self: Sized,
{
    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> StateResponse;
}

/// The message that the integrating module should propagate to `Handler::on_inner`
pub trait ForwardToInner {
    type Msg: Serialize;

    fn msg() -> Self::Msg;
}

pub(crate) fn forward_to_inner<H, ForwardToInnerMsg, SEnum>(
    inner: H,
    response: Binary,
    env: Env,
) -> Result<SEnum>
where
    ForwardToInnerMsg: ForwardToInner,
    SEnum: Handler,
    ResponseDelivery<H, ForwardToInnerMsg>: Into<SEnum::Response>,
{
    let next_state = ResponseDelivery::<H, ForwardToInnerMsg>::new(inner, response);
    next_state
        .enter(env.contract.address)
        .and_then(|msgs| response::res_continue::<_, _, SEnum>(msgs, next_state))
        .into()
}

pub trait TimeAlarm {
    fn setup_alarm(&self, forr: Timestamp) -> DexResult<Batch>;
}
