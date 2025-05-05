use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use sdk::cosmwasm_std::{Env, QuerierWrapper};

use crate::{SwapOutputTask, error::Result, swap::ExactAmountIn};

use crate::{
    SwapTask as SwapTaskT, WithOutputTask,
    impl_::{
        response::{self, Handler, Result as HandlerResult},
        transfer_in_init::TransferInInit,
    },
};

pub struct DecodeThenTransferIn<'resp, SwapTask, SEnum, SwapClient> {
    resp: &'resp [u8],
    _spec: PhantomData<SwapTask>,
    _senum: PhantomData<SEnum>,
    _client: PhantomData<SwapClient>,
}

impl<'resp, SwapTask, SEnum, SwapClient> DecodeThenTransferIn<'resp, SwapTask, SEnum, SwapClient> {
    pub fn from(resp: &'resp [u8]) -> Self {
        Self {
            resp,
            _spec: PhantomData,
            _senum: PhantomData,
            _client: PhantomData,
        }
    }
}
impl<SwapTask, SEnum, SwapClient> WithOutputTask<SwapTask>
    for DecodeThenTransferIn<'_, SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    type Output = Result<TransferInInit<SwapTask, SEnum>>;

    fn on<OutC, SwapOutTask>(self, task: SwapOutTask) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        SwapOutTask: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        let spec = task.into_spec();
        super::decode_response::<OutC, _, SwapClient>(&spec, self.resp)
            .map(|amount_out| TransferInInit::new(spec, amount_out.into()))
    }
}

pub struct DecodeThenFinish<'resp, 'querier, 'env, SwapTask, Handler, SwapClient> {
    resp: &'resp [u8],
    querier: QuerierWrapper<'querier>,
    env: &'env Env,
    _spec: PhantomData<SwapTask>,
    _handler: PhantomData<Handler>,
    _client: PhantomData<SwapClient>,
}

impl<'resp, 'querier, 'env, SwapTask, Handler, SwapClient>
    DecodeThenFinish<'resp, 'querier, 'env, SwapTask, Handler, SwapClient>
{
    pub fn from(resp: &'resp [u8], querier: QuerierWrapper<'querier>, env: &'env Env) -> Self {
        Self {
            resp,
            querier,
            env,
            _spec: PhantomData,
            _handler: PhantomData,
            _client: PhantomData,
        }
    }
}
impl<SwapTask, HandlerT, SwapClient> WithOutputTask<SwapTask>
    for DecodeThenFinish<'_, '_, '_, SwapTask, HandlerT, SwapClient>
where
    SwapTask: SwapTaskT,
    HandlerT: Handler<SwapResult = SwapTask::Result>,
    SwapClient: ExactAmountIn,
{
    type Output = HandlerResult<HandlerT>;

    fn on<OutC, SwapOutTask>(self, task: SwapOutTask) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        SwapOutTask: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        super::decode_response::<OutC, _, SwapClient>(task.as_spec(), self.resp).map_or_else(
            |err| HandlerResult::Continue(Err(err)),
            |amount_out| response::res_finished(task.finish(amount_out, self.env, self.querier)),
        )
    }
}
