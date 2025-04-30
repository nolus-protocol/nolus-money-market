use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use currency::{CurrencyDTO, Group, MemberOf};
use finance::{
    coin::{self, Amount, CoinDTO},
    duration::Duration,
    zero::Zero,
};
use platform::{batch::Batch, trx};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp};

use crate::{
    AnomalyMonitoredTask, AnomalyPolicy, ConnectionParams, Contract, Stage, error::Result,
    swap::ExactAmountIn,
};

use crate::{
    Connectable, ContractInSwap, Enterable, SwapTask as SwapTaskT, TimeAlarm,
    impl_::{
        ForwardToInner,
        response::{self, ContinueResult, Handler, Result as HandlerResult},
        timeout,
        transfer_in_init::TransferInInit,
        trx::SwapTrx,
    },
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de>",
))]
pub struct SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient> {
    spec: SwapTask,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_group: PhantomData<SwapGroup>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    Self: Into<SEnum>,
{
    pub(super) fn new(spec: SwapTask) -> Self {
        Self {
            spec,
            _state_enum: PhantomData,
            _swap_group: PhantomData,
            _swap_client: PhantomData,
        }
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: AnomalyMonitoredTask,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
{
    pub(super) fn enter_state(
        &self,
        _now: Timestamp,
        querier: QuerierWrapper<'_>,
    ) -> Result<Batch> {
        let mut filtered = false;

        let swap_trx = SwapTrx::<'_, '_, '_, <SwapTask::InG as Group>::TopG, _>::new(
            self.spec.dex_account(),
            self.spec.oracle(),
            querier,
        );
        let out_currency = self.spec.out_currency().into_super_group();
        try_filter_fold_coins(
            &self.spec,
            not_out_coins_filter::<_, <SwapTask::InG as Group>::TopG>(&out_currency),
            swap_trx,
            |mut trx, coin_in| {
                filtered = true;
                trx.swap_exact_in::<_, _, SwapClient>(
                    &coin_in,
                    &self.spec.policy().min_output(&coin_in),
                )
                .map(|()| trx)
            },
        )
        .inspect(|_| {
            expect_at_lease_one_filtered(filtered, &out_currency);
        })
        .map(Into::into)
    }

    fn decode_response(&self, resp: &[u8]) -> Result<CoinDTO<SwapTask::OutG>> {
        let out_currency: CurrencyDTO<<SwapTask::InG as Group>::TopG> =
            self.spec.out_currency().into_super_group();
        try_filter_fold_coins(
            &self.spec,
            out_coins_filter(&out_currency),
            Amount::ZERO,
            |total_out, r#in| Ok(total_out + r#in.amount()),
        )
        .and_then(|non_swapped: Amount| {
            trx::decode_msg_responses(resp)
                .map_err(Into::into)
                .and_then(|mut responses| {
                    let mut filtered = false;

                    try_filter_fold_coins(
                        &self.spec,
                        not_out_coins_filter(&out_currency),
                        non_swapped,
                        |total_out, _in| {
                            filtered = true;
                            SwapClient::parse_response(&mut responses)
                                .map(|out| total_out + out)
                                .map_err(Into::into)
                        },
                    )
                    .inspect(|_| {
                        expect_at_lease_one_filtered(filtered, &out_currency);
                    })
                })
        })
        .map(|amount| coin::from_amount_ticker(amount, self.spec.out_currency()))
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: AnomalyMonitoredTask,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
    Self: Handler<Response = SEnum> + Into<SEnum>,
{
    fn retry(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env).into()
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Enterable
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: AnomalyMonitoredTask,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
{
    fn enter(&self, now: Timestamp, querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now, querier)
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Connectable
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn dex(&self) -> &ConnectionParams {
        self.spec.dex_account().dex()
    }
}

impl<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_local::State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg>,
        SwapGroup,
        SwapClient,
    >
where
    SwapTask: AnomalyMonitoredTask,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
    ForwardToInnerMsg: ForwardToInner,
{
    type Response = super::out_local::State<SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg>;
    type SwapResult = SwapTask::Result;

    fn on_response(
        self,
        resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
        self.decode_response(resp.as_slice())
            .map(|amount_out| TransferInInit::new(self.spec, amount_out))
            .and_then(|next_state| {
                next_state
                    .enter(env.block.time, querier)
                    .and_then(|resp| response::res_continue::<_, _, Self>(resp, next_state))
            })
            .into()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.retry(querier, env)
    }
}

impl<OpenIca, SwapTask, SwapGroup, SwapClient, ForwardToInnerMsg, ForwardToInnerContinueMsg> Handler
    for SwapExactIn<
        SwapTask,
        super::out_remote::State<
            OpenIca,
            SwapTask,
            SwapGroup,
            SwapClient,
            ForwardToInnerMsg,
            ForwardToInnerContinueMsg,
        >,
        SwapGroup,
        SwapClient,
    >
where
    SwapTask: AnomalyMonitoredTask,
    SwapGroup: Group,
    SwapClient: ExactAmountIn,
{
    type Response = super::out_remote::State<
        OpenIca,
        SwapTask,
        SwapGroup,
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
        // TODO transfer (downpayment - transferred_and_swapped), i.e. the nls_swap_fee to the profit
        self.decode_response(resp.as_slice()).map_or_else(
            |err| HandlerResult::Continue(Err(err)),
            |amount_out| response::res_finished(self.spec.finish(amount_out, &env, querier)),
        )
    }

    fn on_error(self, _querier: QuerierWrapper<'_>, _env: Env) -> HandlerResult<Self> {
        // self.spec.policy()
        todo!()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    fn heal(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.retry(querier, env)
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> Contract
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
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

impl<SwapTask, SEnum, SwapGroup, SwapClient> Display
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("SwapExactIn at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, SwapGroup, SwapClient> TimeAlarm
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, forr: Timestamp) -> Result<Batch> {
        self.spec.time_alarm().setup_alarm(forr).map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, SwapGroup, SwapClient>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew>
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
where
    Self: Sized,
    SwapExactIn<SwapTaskNew, SEnumNew, SwapGroup, SwapClient>: Into<SEnumNew>,
{
    type Out = SwapExactIn<SwapTaskNew, SEnumNew, SwapGroup, SwapClient>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new(migrate_fn(self.spec))
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, SwapGroup, SwapClient> InspectSpec<SwapTask, R>
    for SwapExactIn<SwapTask, SEnum, SwapGroup, SwapClient>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
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
    move |coin_in| !out_coins_filter::<InG, InOutG>(out_c)(coin_in)
}

fn expect_at_lease_one_filtered<G>(filtered: bool, out_c: &CurrencyDTO<G>)
where
    G: Group,
{
    assert!(filtered, "No coins with currency != {}", out_c)
}
