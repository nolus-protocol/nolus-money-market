use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use remote_lease::response::OperationResponse;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use decode_resp::{DecodeThenFinish, DecodeThenTransferIn};
use encode_req::EncodeRequest;
use finance::instant::Instant;
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
};
use platform::{batch::Batch, remote::ErrorResponse as ICAErrorResponse};
use report_anomaly::ReportAnomalyCmd;
use sdk::cosmwasm_std::{self, Binary, Env, QuerierWrapper};

use crate::{
    AnomalyTreatment, Connectable, ConnectionParams, Contract, ContractInSwap, Enterable, Error,
    RemoteLeaseTransportFactory as RemoteLeaseTransportFactoryT, Stage, SwapTask as SwapTaskT,
    TimeAlarm, TransportOutFactory as TransportOutFactoryT,
    error::Result,
    impl_::{
        ForwardToInner,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
    },
};

#[cfg(feature = "migration")]
use super::migration::{_InspectSpec, _MigrateSpec};
use cw_time::IntoInstant;

mod decode_resp;
mod encode_req;
mod report_anomaly;

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize,
                    RemoteLeaseTransportFactory: Serialize",
    deserialize = "SwapTask: Deserialize<'de>,
                    RemoteLeaseTransportFactory: Deserialize<'de>",
))]
pub struct SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory> {
    spec: SwapTask,
    transport_factory: RemoteLeaseTransportFactory,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    Self: Into<SEnum>,
{
    pub(super) fn new(spec: SwapTask, transport_factory: RemoteLeaseTransportFactory) -> Self {
        Self {
            spec,
            transport_factory,
            _state_enum: PhantomData,
        }
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
{
    pub(super) fn enter_state(&self, now: Instant, querier: QuerierWrapper<'_>) -> Result<Batch> {
        let transport = self.transport_factory.transport(&self.spec, now);
        self.spec.with_slippage_calc(EncodeRequest::<
            '_,
            '_,
            _,
            RemoteLeaseTransportFactory::TransportImpl<'_>,
        >::from(&self.spec, transport, querier))
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory>
    SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    fn handle_error(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        match self.spec.into_output_task(ReportAnomalyCmd::default()) {
            AnomalyTreatment::Retry(spec) => {
                let swap_exact_in = SwapExactIn::new(spec, self.transport_factory);
                swap_exact_in
                    .enter(env.block.time.into_instant(), querier)
                    .and_then(|batch| response::res_continue::<_, _, Self>(batch, swap_exact_in))
                    .into()
            }
            AnomalyTreatment::Exit(result) => response::res_finished(result),
        }
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Enterable
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
{
    fn enter(&self, now: Instant, querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now, querier)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Connectable
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory: RemoteLeaseTransportFactoryT,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

impl<SwapTask, TransportOutFactory, RemoteLeaseTransportFactory, ForwardToInnerMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_local::State<
            SwapTask,
            TransportOutFactory,
            RemoteLeaseTransportFactory,
            ForwardToInnerMsg,
        >,
        RemoteLeaseTransportFactory,
    >
where
    SwapTask: SwapTaskT,
    TransportOutFactory: TransportOutFactoryT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
    ForwardToInnerMsg: ForwardToInner,
{
    type Response = super::out_local::State<
        SwapTask,
        TransportOutFactory,
        RemoteLeaseTransportFactory,
        ForwardToInnerMsg,
    >;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &sdk::cosmwasm_std::MessageInfo,
    ) -> crate::error::Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    fn on_response(
        self,
        resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.spec
            .into_output_task(
                DecodeThenTransferIn::<'_, _, _, RemoteLeaseTransportFactory>::from(
                    resp.as_slice(),
                ),
            )
            .and_then(|next_state| {
                next_state
                    .enter(env.block.time.into_instant(), querier)
                    .and_then(|resp| response::res_continue::<_, _, Self>(resp, next_state))
            })
            .into()
    }

    fn on_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.handle_error(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }
}

impl<SwapTask, TransportOutFactory, RemoteLeaseTransportFactory, ForwardToInnerMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_remote::State<
            SwapTask,
            TransportOutFactory,
            RemoteLeaseTransportFactory,
            ForwardToInnerMsg,
        >,
        RemoteLeaseTransportFactory,
    >
where
    SwapTask: SwapTaskT,
    RemoteLeaseTransportFactory:
        RemoteLeaseTransportFactoryT<TopG = <SwapTask::InG as Group>::TopG>,
{
    type Response = super::out_remote::State<
        SwapTask,
        TransportOutFactory,
        RemoteLeaseTransportFactory,
        ForwardToInnerMsg,
    >;
    type SwapResult = SwapTask::Result;

    fn authz_remote_callback(
        &self,
        querier: QuerierWrapper<'_>,
        info: &sdk::cosmwasm_std::MessageInfo,
    ) -> crate::error::Result<()> {
        self.spec.authz_remote_callback(querier, info)
    }

    fn on_response(
        self,
        resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.spec.into_output_task(DecodeThenFinish::<
            '_,
            '_,
            '_,
            _,
            _,
            RemoteLeaseTransportFactory,
        >::from(resp.as_slice(), querier, &env))
    }

    fn on_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.handle_error(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Contract
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Instant,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec.state(Stage::Swap, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> Display
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("SwapExactIn at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, RemoteLeaseTransportFactory> TimeAlarm
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, forr: Instant) -> Result<Batch> {
        self.spec.time_alarm().setup_alarm(forr).map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, RemoteLeaseTransportFactory>
    _MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
where
    Self: Sized,
    SwapExactIn<SwapTaskNew, SEnumNew, RemoteLeaseTransportFactory>: Into<SEnumNew>,
{
    type Out = SwapExactIn<SwapTaskNew, SEnumNew, RemoteLeaseTransportFactory>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec), self.transport_factory)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, RemoteLeaseTransportFactory> _InspectSpec<SwapTask, R>
    for SwapExactIn<SwapTask, SEnum, RemoteLeaseTransportFactory>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
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
    // `amount_out` is the full post-swap output: coins already in the output
    // currency are excluded from the request (`not_out_coins_filter`) and folded
    // in by the counterparty, so no non-swapped term is re-added here.
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

fn try_filter_fold_coins<SwapTask, FilterFn, Acc, FoldFn>(
    spec: &SwapTask,
    filter: FilterFn,
    init: Acc,
    fold: FoldFn,
) -> Result<Acc>
where
    SwapTask: SwapTaskT,
    FilterFn: Fn(&CoinDTO<SwapTask::InG>) -> bool,
    FoldFn: FnMut(Acc, CoinDTO<SwapTask::InG>) -> Result<Acc>,
{
    spec.coins().into_iter().filter(filter).try_fold(init, fold)
}

fn out_coins_filter<InG, InOutG>(out_c: &CurrencyDTO<InOutG>) -> impl Fn(&CoinDTO<InG>) -> bool
where
    InG: Group + MemberOf<InOutG>,
    InOutG: Group,
{
    move |coin_in| {
        coin_in
            .into_super_group::<InOutG>()
            .of_currency_dto(out_c)
            .is_ok()
    }
}

fn not_out_coins_filter<InG, InOutG>(out_c: &CurrencyDTO<InOutG>) -> impl Fn(&CoinDTO<InG>) -> bool
where
    InG: Group + MemberOf<InOutG>,
    InOutG: Group,
{
    |coin_in| !out_coins_filter::<InG, InOutG>(out_c)(coin_in)
}
