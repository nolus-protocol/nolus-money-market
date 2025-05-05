use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use decode_resp::{DecodeThenFinish, DecodeThenTransferIn};
use encode_req::EncodeRequest;
use report_anomaly::ReportAnomalyCmd;
use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, CurrencyDef, Group, MemberOf};
use finance::{
    coin::{Coin, CoinDTO},
    duration::Duration,
    zero::Zero,
};
use platform::{batch::Batch, trx};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp};

use crate::{
    AnomalyTreatment, ConnectionParams, Contract, Stage, error::Result, swap::ExactAmountIn,
};

use crate::{
    Connectable, ContractInSwap, Enterable, SwapTask as SwapTaskT, TimeAlarm,
    impl_::{
        ForwardToInner,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
    },
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};

mod decode_resp;
mod encode_req;
mod report_anomaly;

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de>",
))]
pub struct SwapExactIn<SwapTask, SEnum, SwapClient> {
    spec: SwapTask,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, SwapClient> SwapExactIn<SwapTask, SEnum, SwapClient>
where
    Self: Into<SEnum>,
{
    pub(super) fn new(spec: SwapTask) -> Self {
        Self {
            spec,
            _state_enum: PhantomData,
            _swap_client: PhantomData,
        }
    }
}

impl<SwapTask, SEnum, SwapClient> SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    pub(super) fn enter_state(
        &self,
        _now: Timestamp,
        querier: QuerierWrapper<'_>,
    ) -> Result<Batch> {
        self.spec
            .with_slippage_calc(EncodeRequest::<'_, _, SwapClient>::from(querier))
    }
}

impl<SwapTask, SEnum, SwapClient> SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    Self: Handler<Response = SEnum, SwapResult = SwapTask::Result> + Into<SEnum>,
{
    fn handle_error(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        match self.spec.into_output_task(ReportAnomalyCmd::default()) {
            AnomalyTreatment::Retry(spec) => {
                let swap_exact_in = SwapExactIn::new(spec);
                swap_exact_in
                    .enter(env.block.time, querier)
                    .and_then(|batch| response::res_continue::<_, _, Self>(batch, swap_exact_in))
                    .into()
            }
            AnomalyTreatment::Exit(result) => response::res_finished(result),
        }
    }
}

impl<SwapTask, SEnum, SwapClient> SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    Self: Handler<Response = SEnum> + Into<SEnum>,
{
    fn retry(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env).into()
    }
}

impl<SwapTask, SEnum, SwapClient> Enterable for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    fn enter(&self, now: Timestamp, querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now, querier)
    }
}

impl<SwapTask, SEnum, SwapClient> Connectable for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

impl<SwapTask, SwapClient, ForwardToInnerMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_local::State<SwapTask, SwapClient, ForwardToInnerMsg>,
        SwapClient,
    >
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    ForwardToInnerMsg: ForwardToInner,
{
    type Response = super::out_local::State<SwapTask, SwapClient, ForwardToInnerMsg>;
    type SwapResult = SwapTask::Result;

    fn on_response(
        self,
        resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.spec
            .into_output_task(DecodeThenTransferIn::<'_, _, _, SwapClient>::from(
                resp.as_slice(),
            ))
            .and_then(|next_state| {
                next_state
                    .enter(env.block.time, querier)
                    .and_then(|resp| response::res_continue::<_, _, Self>(resp, next_state))
            })
            .into()
    }

    fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.handle_error(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.retry(querier, env)
    }
}

impl<OpenIca, SwapTask, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_remote::State<
            OpenIca,
            SwapTask,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >,
        SwapClient,
    >
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    type Response = super::out_remote::State<
        OpenIca,
        SwapTask,
        SwapClient,
        ForwardToInnerMsg,
        ForwardToInnerContinueMsg,
    >;
    type SwapResult = SwapTask::Result;

    fn on_response(
        self,
        resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        self.spec
            .into_output_task(DecodeThenFinish::<'_, '_, '_, _, _, SwapClient>::from(
                resp.as_slice(),
                querier,
                &env,
            ))
    }

    fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.handle_error(querier, env)
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.retry(querier, env)
    }
}

impl<SwapTask, SEnum, SwapClient> Contract for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT + ContractInSwap<StateResponse = <SwapTask as SwapTaskT>::StateResponse>,
{
    type StateResponse = <SwapTask as SwapTaskT>::StateResponse;

    fn state(
        self,
        now: Timestamp,
        due_projection: Duration,
        querier: QuerierWrapper<'_>,
    ) -> Self::StateResponse {
        self.spec.state(Stage::Swap, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum, SwapClient> Display for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("SwapExactIn at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, SwapClient> TimeAlarm for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, forr: Timestamp) -> Result<Batch> {
        self.spec.time_alarm().setup_alarm(forr).map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, SwapClient>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew> for SwapExactIn<SwapTask, SEnum, SwapClient>
where
    Self: Sized,
    SwapExactIn<SwapTaskNew, SEnumNew, SwapClient>: Into<SEnumNew>,
{
    type Out = SwapExactIn<SwapTaskNew, SEnumNew, SwapClient>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec))
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, SwapClient> InspectSpec<SwapTask, R>
    for SwapExactIn<SwapTask, SEnum, SwapClient>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}

fn decode_response<OutC, SwapTask, SwapClient>(spec: &SwapTask, resp: &[u8]) -> Result<Coin<OutC>>
where
    OutC: CurrencyDef,
    OutC::Group: MemberOf<<SwapTask::OutG as Group>::TopG>,
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
{
    let out_currency = OutC::dto().into_super_group();
    try_filter_fold_coins(
        spec,
        out_coins_filter(&out_currency),
        Coin::<OutC>::ZERO,
        |total_out, inn| {
            Ok(total_out
                + inn
                    .into_super_group::<<SwapTask::OutG as Group>::TopG>()
                    .as_specific(OutC::dto()))
        },
    )
    .and_then(|non_swapped| {
        trx::decode_msg_responses(resp)
            .map_err(Into::into)
            .and_then(|mut responses| {
                let mut filtered = false;

                try_filter_fold_coins(
                    spec,
                    not_out_coins_filter(&out_currency),
                    non_swapped,
                    |total_out, _in| {
                        filtered = true;
                        SwapClient::parse_response(&mut responses)
                            .map(|out| total_out + out.into())
                            .map_err(Into::into)
                    },
                )
                .inspect(|_| {
                    expect_at_lease_one_filtered(filtered, &out_currency);
                })
            })
    })
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

fn expect_at_lease_one_filtered<G>(filtered: bool, out_c: &CurrencyDTO<G>)
where
    G: Group,
{
    assert!(filtered, "No coins with currency != {}", out_c)
}
