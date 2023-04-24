use finance::coin::CoinDTO;
use sdk::cosmwasm_std::Binary;
use serde::{Deserialize, Serialize};

use crate::{
    error::Result,
    response::{ContinueResult, Handler, Result as HandlerResult},
    swap_task::SwapTask as SwapTaskT,
    timeout,
    trx::IBC_TIMEOUT,
    ConnectionParams, Contract, ContractInSwap, DexConnectable, Enterable, TransferInInitState,
};
use platform::batch::Batch;
use sdk::cosmwasm_std::{Deps, Env, QuerierWrapper, Timestamp};

use super::transfer_in_finish::TransferInFinish;

/// Transfer in a coin from DEX
///
#[derive(Serialize, Deserialize)]
pub struct TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
{
    spec: SwapTask,
    amount_in: CoinDTO<SwapTask::OutG>,
}

impl<SwapTask> TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
{
    pub fn new(spec: SwapTask, amount_in: CoinDTO<SwapTask::OutG>) -> Self {
        Self { spec, amount_in }
    }
}

impl<SwapTask> TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
{
    fn enter_state(&self, now: Timestamp) -> Result<Batch> {
        let mut sender = self.spec.dex_account().transfer_from(now);
        sender.send(&self.amount_in)?;
        Ok(sender.into())
    }
}

impl<SwapTask> TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
    SwapTask::OutG: Clone,
{
    fn on_response(self, deps: Deps<'_>, env: Env) -> HandlerResult<Self> {
        let finish = TransferInFinish::new(self.spec, self.amount_in, env.block.time + IBC_TIMEOUT);
        finish.try_complete(deps, env).map_into()
    }
}

impl<SwapTask> DexConnectable for TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

impl<SwapTask> Enterable for TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
{
    fn enter(&self, now: Timestamp, _querier: &QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now)
    }
}

impl<SwapTask> Handler for TransferInInit<SwapTask>
where
    SwapTask: SwapTaskT,
    SwapTask::OutG: Clone,
{
    type Response = super::out_local::State<SwapTask>;
    type SwapResult = SwapTask::Result;

    fn on_response(self, _data: Binary, deps: Deps<'_>, env: Env) -> HandlerResult<Self> {
        self.on_response(deps, env)
    }

    fn on_timeout(self, _deps: Deps<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        let timealarms = self.spec.time_alarm().clone();
        timeout::on_timeout_repair_channel(self, state_label, timealarms, env)
    }
}

impl<SwapTask> Contract for TransferInInit<SwapTask>
where
    SwapTask:
        SwapTaskT + ContractInSwap<TransferInInitState, <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(self, now: Timestamp, querier: &QuerierWrapper<'_>) -> Self::StateResponse {
        self.spec.state(now, querier)
    }
}
