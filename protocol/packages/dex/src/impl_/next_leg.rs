use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::SwapTask as SwapTaskT;

use super::response::{Handler, Result as HandlerResult};

/// Successor leg [`TransferOut`](super::transfer_out::TransferOut) hands off
/// to once the last transfer acknowledgment arrives
pub trait NextLeg<SwapTask>
where
    SwapTask: SwapTaskT,
    Self: Handler<SwapResult = SwapTask::Result>,
{
    /// Construct the leg over the completed predecessor's swap task and
    /// produce its first response
    fn enter_from(spec: SwapTask, querier: QuerierWrapper<'_>, env: &Env) -> HandlerResult<Self>;
}
