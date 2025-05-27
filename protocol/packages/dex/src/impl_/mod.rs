use sdk::cosmwasm_std::{Binary, Env};

use crate::{ContinueResult, ForwardToInner, Handler, Result, SwapTask, response};

pub use self::{
    ica_connector::IcaConnector,
    out_local::{
        StartLocalLocalState, StartTransferInState, State as StateLocalOut, start_local_local,
        start_remote_local,
    },
    out_remote::{StartLocalRemoteState, State as StateRemoteOut, start as start_local_remote},
    resp_delivery::{ICAOpenResponseDelivery, ResponseDelivery},
    slippage::{AcceptAnyNonZeroSwap, Calculator as AcceptUpToMaxSlippage, MaxSlippage},
    swap_exact_in::SwapExactIn,
    transfer_in_finish::TransferInFinish,
    transfer_in_init::TransferInInit,
    transfer_out::TransferOut,
};

mod ica_connector;
#[cfg(feature = "migration")]
mod migration;
mod out_local;
mod out_remote;
mod resp_delivery;
mod slippage;
mod swap_exact_in;
mod timeout;
mod transfer_in;
mod transfer_in_finish;
mod transfer_in_init;
mod transfer_out;
mod trx;

pub type TransferOutRespDelivery<SwapTask, SEnum, SwapClient, ForwardToInnerMsg> =
    ResponseDelivery<TransferOut<SwapTask, SEnum, SwapClient>, ForwardToInnerMsg>;

pub type SwapExactInRespDelivery<SwapTask, SEnum, SwapClient, ForwardToInnerMsg> =
    ResponseDelivery<SwapExactIn<SwapTask, SEnum, SwapClient>, ForwardToInnerMsg>;

pub type TransferInInitRespDelivery<SwapTask, SEnum, ForwardToInnerMsg> =
    ResponseDelivery<TransferInInit<SwapTask, SEnum>, ForwardToInnerMsg>;

fn forward_to_inner<H, ForwardToInnerMsg, SEnum>(
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

fn forward_to_inner_ica<H, ForwardToInnerContinueMsg, SEnum>(
    inner: H,
    counterparty_version: String,
    env: Env,
) -> ContinueResult<SEnum>
where
    ForwardToInnerContinueMsg: ForwardToInner,
    SEnum: Handler,
    ICAOpenResponseDelivery<H, ForwardToInnerContinueMsg>: Into<SEnum::Response>,
{
    let next_state =
        ICAOpenResponseDelivery::<H, ForwardToInnerContinueMsg>::new(inner, counterparty_version);
    next_state
        .enter(env.contract.address)
        .and_then(|msgs| response::res_continue::<_, _, SEnum>(msgs, next_state))
}
