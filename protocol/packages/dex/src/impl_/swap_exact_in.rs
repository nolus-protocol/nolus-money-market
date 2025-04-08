use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use oracle::stub::SwapPath;
use serde::{Deserialize, Serialize};

use currency::Group;
use finance::{
    coin::{self, Amount, CoinDTO},
    duration::Duration,
    zero::Zero,
};
use platform::{batch::Batch, trx};
use sdk::{
    cosmos_sdk_proto::Any,
    cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp},
};

use crate::{
    AnomalyMonitoredTask, AnomalyPolicy, ConnectionParams, Contract, Stage,
    error::{Error, Result},
    swap::ExactAmountIn,
};

#[cfg(debug_assertions)]
use crate::IterState;
use crate::{
    CoinVisitor, Connectable, ContractInSwap, Enterable, IterNext, SwapTask as SwapTaskT,
    TimeAlarm,
    impl_::{
        ForwardToInner,
        filter::CurrencyFilter,
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
        let swap_trx = SwapTrx::new(self.spec.dex_account(), self.spec.oracle(), querier);
        // TODO apply nls_swap_fee on the downpayment only!
        struct SwapWorker<'ica, 'swap_path, 'querier, 'task, SwapPathImpl, SwapTask, SwapClient>(
            SwapTrx<'ica, 'swap_path, 'querier, SwapTask::InOutG, SwapPathImpl>,
            &'task SwapTask,
            PhantomData<SwapClient>,
        )
        where
            SwapTask: AnomalyMonitoredTask;

        impl<SwapPathImpl, SwapTask, SwapClient> CoinVisitor
            for SwapWorker<'_, '_, '_, '_, SwapPathImpl, SwapTask, SwapClient>
        where
            SwapPathImpl: SwapPath<SwapTask::InOutG>,
            SwapTask: AnomalyMonitoredTask,
            SwapClient: ExactAmountIn,
        {
            type GIn = SwapTask::InG;

            type Result = IterNext;

            type Error = Error;

            fn visit(&mut self, coin: &CoinDTO<Self::GIn>) -> Result<Self::Result> {
                let min_out = self.1.policy().min_output(coin);

                self.0
                    .swap_exact_in::<_, SwapTask::InG, SwapTask::OutG, SwapClient>(coin, &min_out)
                    .map(|()| IterNext::Continue)
            }
        }

        let mut swapper = SwapWorker(swap_trx, &self.spec, PhantomData::<SwapClient>);

        let mut filtered_swapper =
            CurrencyFilter::<_, _, _>::new(&mut swapper, self.spec.out_currency());

        #[cfg_attr(not(debug_assertions), expect(unused_variables))]
        let res = self.spec.on_coins(&mut filtered_swapper)?;

        #[cfg(debug_assertions)]
        self.postcondition_check(&filtered_swapper, res);

        Ok(swapper.0.into())
    }

    fn decode_response(&self, resp: &[u8], spec: &SwapTask) -> Result<CoinDTO<SwapTask::OutG>> {
        struct ExactInResponse<I, SwapIn, SwapClient>(
            I,
            Amount,
            PhantomData<SwapIn>,
            PhantomData<SwapClient>,
        );

        impl<I, SwapIn, SwapClient> CoinVisitor for ExactInResponse<I, SwapIn, SwapClient>
        where
            SwapIn: Group,
            I: Iterator<Item = Any>,
            SwapClient: ExactAmountIn,
        {
            type GIn = SwapIn;

            type Result = IterNext;

            type Error = Error;

            fn visit(&mut self, _coin: &CoinDTO<Self::GIn>) -> Result<Self::Result> {
                SwapClient::parse_response(&mut self.0)
                    .inspect(|&amount| self.1 += amount)
                    .map(|_| IterNext::Continue)
                    .map_err(Into::into)
            }
        }

        let mut resp = ExactInResponse(
            trx::decode_msg_responses(resp)?,
            Amount::ZERO,
            PhantomData::<SwapTask::InG>,
            PhantomData::<SwapClient>,
        );

        let mut filtered_resp = CurrencyFilter::new(&mut resp, self.spec.out_currency());

        #[cfg_attr(not(debug_assertions), expect(unused_variables))]
        let res = self.spec.on_coins(&mut filtered_resp)?;

        #[cfg(debug_assertions)]
        self.postcondition_check(&filtered_resp, res);

        Ok(coin::from_amount_ticker(
            filtered_resp.filtered() + resp.1,
            spec.out_currency(),
        ))
    }

    #[cfg(debug_assertions)]
    fn postcondition_check<V>(
        &self,
        filter: &CurrencyFilter<'_, V, SwapTask::InG, SwapTask::OutG>,
        res: IterState,
    ) where
        V: CoinVisitor,
    {
        debug_assert!(
            filter.passed_any(),
            "No coins with currency != {}",
            self.spec.out_currency()
        );
        debug_assert_eq!(res, IterState::Complete);
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
        self.decode_response(resp.as_slice(), &self.spec)
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
        self.decode_response(resp.as_slice(), &self.spec)
            .map_or_else(
                |err| HandlerResult::Continue(Err(err)),
                |amount_out| response::res_finished(self.spec.finish(amount_out, &env, querier)),
            )
    }

    fn on_error(self, _querier: QuerierWrapper<'_>, _env: Env) -> ContinueResult<Self> {
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
