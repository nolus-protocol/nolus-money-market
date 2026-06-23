use sdk::cosmwasm_std::{Binary, Env};

use crate::{ForwardToInner, Handler, Result, SwapTask, response};

pub use self::{
    drain::{StartDrainState, State as StateDrain, start as start_drain},
    funding::{Funding, FundingClient},
    funds_arrival::FundsArrival,
    ica_connector::IcaConnector,
    out_fund_remote::{StartFundRemoteState, State as StateFundRemote, start as start_fund_remote},
    out_local::{
        StartLocalLocalState, StartTransferInState, State as StateLocalOut, start_local_local,
        start_remote_local,
    },
    out_swap::{StartOutSwapState, State as StateOutSwap, start as start_out_swap},
    remote_swap::{RemoteSwap, RemoteSwapClient},
    remote_swap_only::{StartSwapState, State as StateSwap, start as start_swap},
    remote_transfer_out::{DrainStage, RemoteTransferOut, RemoteTransferOutTask},
    resp_delivery::ResponseDelivery,
    slippage::{AcceptAnyNonZeroSwap, Calculator as AcceptUpToMaxSlippage, MaxSlippage},
    slippage_anomaly::SlippageAnomaly,
    swap_exact_in::SwapExactIn,
    transfer_in_finish::TransferInFinish,
    transfer_in_init::TransferInInit,
    transfer_out::TransferOut,
};

mod drain;
mod funding;
mod funds_arrival;
mod ica_connector;
#[cfg(feature = "migration")]
mod migration;
mod next_leg;
mod out_fund_remote;
mod out_local;
mod out_swap;
mod remote_swap;
mod remote_swap_only;
mod remote_transfer_out;
mod resp_delivery;
mod slippage;
mod slippage_anomaly;
mod swap_exact_in;
mod timeout;
mod transfer_in;
mod transfer_in_finish;
mod transfer_in_init;
mod transfer_out;
mod trx;

pub type TransferOutRespDelivery<
    SwapTask,
    SEnum,
    SwapClient,
    ForwardToInnerMsg,
    NextLeg = SwapExactIn<SwapTask, SEnum, SwapClient>,
> = ResponseDelivery<TransferOut<SwapTask, SEnum, SwapClient, NextLeg>, ForwardToInnerMsg>;

pub type FundingRespDelivery<SwapTask, SEnum, ForwardToInnerMsg, NextLeg> =
    ResponseDelivery<Funding<SwapTask, SEnum, NextLeg>, ForwardToInnerMsg>;

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
