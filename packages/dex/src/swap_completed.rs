use std::marker::PhantomData;

use finance::coin::CoinDTO;
use serde::{Deserialize, Serialize};

use crate::{swap_task::SwapTask as SwapTaskT, Handler, Error};

#[derive(Serialize, Deserialize)]
pub struct Completed<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    task: SwapTask,
    amount_out: CoinDTO<SwapTask::OutG>,
    _state_enum: PhantomData<SEnum>,
}

impl<SwapTask, SEnum> Completed<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    pub(super) fn new(task: SwapTask, amount_out: CoinDTO<SwapTask::OutG>) -> Self {
        Self {
            task,
            amount_out,
            _state_enum: PhantomData,
        }
    }
}

impl<SwapTask, SEnum> Handler for Completed<SwapTask, SEnum>
where
    SwapTask: SwapTaskT,
{
    type Response = SEnum;
    type Error = Error;
}
