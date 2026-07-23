use std::marker::PhantomData;

use currency::{CurrencyDef, Group, MemberOf};
use finance::coin::Coin;
use finance::zero::Zero;
use remote_lease::response::OperationResponse;
use sdk::cosmwasm_std::{self, Env, QuerierWrapper};

use crate::{
    Error, RemoteLeaseTransportFactory as RemoteLeaseTransportFactoryT, SwapOutputTask,
    error::Result,
};

use crate::{
    SwapTask as SwapTaskT, WithOutputTask,
    impl_::{
        response::{self, Handler, Result as HandlerResult},
        transfer_in_init::TransferInInit,
    },
};

pub struct DecodeThenTransferIn<'resp, SwapTask, SEnum, RemoteLeaseTransportFactory> {
    resp: &'resp [u8],
    _spec: PhantomData<SwapTask>,
    _senum: PhantomData<SEnum>,
    _factory: PhantomData<RemoteLeaseTransportFactory>,
}

impl<'resp, SwapTask, SEnum, RemoteLeaseTransportFactory>
    DecodeThenTransferIn<'resp, SwapTask, SEnum, RemoteLeaseTransportFactory>
{
    pub fn from(resp: &'resp [u8]) -> Self {
        Self {
            resp,
            _spec: PhantomData,
            _senum: PhantomData,
            _factory: PhantomData,
        }
    }
}
impl<SwapTask, SEnum, RemoteLeaseTransportFactory> WithOutputTask<SwapTask>
    for DecodeThenTransferIn<'_, SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory: RemoteLeaseTransportFactoryT,
{
    type Output = Result<TransferInInit<SwapTask, SEnum>>;

    fn on<OutC, SwapOutTask>(self, task: SwapOutTask) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        SwapOutTask: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        let spec = task.into_spec();
        total_output::<OutC, _>(&spec, self.resp)
            .map(|amount_out| TransferInInit::new(spec, amount_out.into()))
    }
}

pub struct DecodeThenFinish<'resp, 'querier, 'env, SwapTask, Handler, RemoteLeaseTransportFactory> {
    resp: &'resp [u8],
    querier: QuerierWrapper<'querier>,
    env: &'env Env,
    _spec: PhantomData<SwapTask>,
    _handler: PhantomData<Handler>,
    _factory: PhantomData<RemoteLeaseTransportFactory>,
}

impl<'resp, 'querier, 'env, SwapTask, Handler, RemoteLeaseTransportFactory>
    DecodeThenFinish<'resp, 'querier, 'env, SwapTask, Handler, RemoteLeaseTransportFactory>
{
    pub fn from(resp: &'resp [u8], querier: QuerierWrapper<'querier>, env: &'env Env) -> Self {
        Self {
            resp,
            querier,
            env,
            _spec: PhantomData,
            _handler: PhantomData,
            _factory: PhantomData,
        }
    }
}
impl<SwapTask, HandlerT, RemoteLeaseTransportFactory> WithOutputTask<SwapTask>
    for DecodeThenFinish<'_, '_, '_, SwapTask, HandlerT, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    HandlerT: Handler<SwapResult = SwapTask::Result>,
    RemoteLeaseTransportFactory: RemoteLeaseTransportFactoryT,
{
    type Output = HandlerResult<HandlerT>;

    fn on<OutC, SwapOutTask>(self, task: SwapOutTask) -> Self::Output
    where
        OutC: CurrencyDef,
        OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG> + MemberOf<SwapTask::OutG>,
        SwapOutTask: SwapOutputTask<SwapTask, OutC = OutC>,
    {
        total_output(task.as_spec(), self.resp).map_or_else(
            |err| HandlerResult::Continue(Err(err)),
            |amount_out| response::res_finished(task.finish(amount_out, self.env, self.querier)),
        )
    }
}

fn total_output<OutC, SwapTask>(spec: &SwapTask, resp: &[u8]) -> Result<Coin<OutC>>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG>,
    SwapTask: SwapTaskT,
{
    non_swapped_input(spec).and_then(|non_swapped| {
        decode_swap_response::<_, SwapTask>(resp).and_then(|swapped| {
            non_swapped
                .checked_add(swapped)
                .ok_or_else(|| Error::Overflow("calculating the total output"))
        })
    })
}

fn non_swapped_input<OutC, SwapTask>(spec: &SwapTask) -> Result<Coin<OutC>>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG>,
    SwapTask: SwapTaskT,
{
    let out_currency = OutC::dto().into_super_group();
    super::try_filter_fold_coins(
        spec,
        super::out_coins_filter(&out_currency),
        Coin::<OutC>::ZERO,
        |total_out, inn| {
            Ok(total_out
                + inn
                    .into_super_group::<<SwapTask::OutG as Group>::TopG>()
                    .as_specific(OutC::dto()))
        },
    )
}

fn decode_swap_response<OutC, SwapTask>(resp: &[u8]) -> Result<Coin<OutC>>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG>,
    SwapTask: SwapTaskT,
{
    let out_c_dto = OutC::dto();
    let res: OperationResponse<<SwapTask::OutG as Group>::TopG> =
        cosmwasm_std::from_json(resp).map_err(platform::error::Error::Deserialization)?;
    // `amount_out` covers only the swapped coins: coins already in the output
    // currency are excluded from the request (`not_out_coins_filter`) and are
    // NOT returned by the counterparty, so `total_output` re-adds them on the
    // Nolus side via `non_swapped_input`.
    match res {
        OperationResponse::Swap(swap_resp) => Ok(swap_resp.amount_out),
        _ => Err(Error::NotSwapResponse(format!("{res:?}"))),
    }
    .and_then(|amount_dto_out| {
        amount_dto_out
            .of_currency_dto(out_c_dto)
            .map_err(|err| {
                Error::IncorrectSwapOutCurrency(
                    amount_dto_out.to_string(),
                    out_c_dto.to_string(),
                    err,
                )
            })
            .map(|()| amount_dto_out)
    })
    .map(|amount_dto_out| amount_dto_out.as_specific(out_c_dto))
}
