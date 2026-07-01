use sdk::cosmwasm_std::{Binary, Env};

use crate::{ForwardToInner, Handler, Result, response};

pub use self::{
    drain::{StartDrainState, State as StateDrain, start as start_drain},
    funding::{Funding, FundingClient},
    funds_arrival::FundsArrival,
    out_fund_remote::{StartFundRemoteState, State as StateFundRemote, start as start_fund_remote},
    remote_swap::{RemoteSwap, RemoteSwapClient},
    remote_swap_only::{StartSwapState, State as StateSwap, start as start_swap},
    remote_transfer_out::{DrainStage, RemoteTransferOut, RemoteTransferOutTask},
    resp_delivery::ResponseDelivery,
    slippage::{AcceptAnyNonZeroSwap, Calculator as AcceptUpToMaxSlippage, MaxSlippage},
    slippage_anomaly::SlippageAnomaly,
};

mod drain;
mod funding;
mod funds_arrival;
mod next_leg;
mod out_fund_remote;
mod remote_swap;
mod remote_swap_only;
mod remote_transfer_out;
mod resp_delivery;
mod slippage;
mod slippage_anomaly;
mod timeout;
mod transfer_in;

pub type FundingRespDelivery<SwapTask, SEnum, ForwardToInnerMsg, NextLeg> =
    ResponseDelivery<Funding<SwapTask, SEnum, NextLeg>, ForwardToInnerMsg>;

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
