use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};

use crate::{SwapOutputTask, error::Result, swap::ExactAmountIn};

use crate::{SwapTask as SwapTaskT, WithOutputTask, impl_::transfer_in_init::TransferInInit};

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
