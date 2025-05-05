use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use finance::{coin::CoinDTO, duration::Duration};
use platform::{
    batch::{Batch, Emitter},
    message::Response as MessageResponse,
};
use sdk::cosmwasm_std::{Binary, Env, QuerierWrapper, Timestamp};

use crate::{
    CoinsNb, Contract, ContractInSwap, Enterable, Stage, SwapTask as SwapTaskT, TimeAlarm,
    error::Result, swap::ExactAmountIn,
};

#[cfg(feature = "migration")]
use super::migration::{InspectSpec, MigrateSpec};
use super::{
    response::{self, ContinueResult, Handler, Result as HandlerResult},
    swap_exact_in::SwapExactIn,
    timeout,
    trx::TransferOutTrx,
};

/// Transfer out a list of coins to DEX
///
/// Supports up to `CoinsNb::MAX` number of coins.
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "SwapTask: Serialize",
    deserialize = "SwapTask: Deserialize<'de>",
))]
pub struct TransferOut<SwapTask, SEnum, SwapClient> {
    spec: SwapTask,
    coin_index: CoinsNb,
    last_coin_index: CoinsNb,
    #[serde(skip)]
    _state_enum: PhantomData<SEnum>,
    #[serde(skip)]
    _swap_client: PhantomData<SwapClient>,
}

impl<SwapTask, SEnum, SwapClient> TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    pub fn new(spec: SwapTask) -> Self {
        let first_index = Default::default();
        let last_coin_index = Self::last_coin_index(&spec);
        Self::new_with_index(spec, first_index, last_coin_index)
    }

    fn new_with_index(spec: SwapTask, coin_index: CoinsNb, last_coin_index: CoinsNb) -> Self {
        dbg!(coin_index, last_coin_index);
        debug_assert!(dbg!(coin_index <= last_coin_index));
        Self {
            spec,
            coin_index,
            last_coin_index,
            _state_enum: PhantomData,
            _swap_client: PhantomData,
        }
    }

    fn last_coin_index(spec: &SwapTask) -> CoinsNb {
        spec.coins()
            .into_iter()
            .count()
            .checked_sub(1)
            .expect("The swap task did not provide any coins!")
            .try_into()
            .expect("Functionality doesn't support this many coins!")
    }

    fn next(self) -> Self {
        debug_assert!(!self.last_coin());

        let next_index = self.coin_index.checked_add(1).expect(
            "the method contract precondition `!self.last_coin()` should have been respected",
        );

        Self::new_with_index(self.spec, next_index, self.last_coin_index)
    }

    fn last_coin(&self) -> bool {
        debug_assert!(self.coin_index <= self.last_coin_index);
        self.coin_index == self.last_coin_index
    }

    fn enter_state(&self, now: Timestamp) -> Result<Batch> {
        let mut trx = TransferOutTrx::new(self.spec.dex_account(), now);

        trx.send(&self.current_coin()).map(|()| trx.into())
    }

    fn current_coin(&self) -> CoinDTO<SwapTask::InG> {
        self.spec
            .coins()
            .into_iter()
            .nth(self.coin_index.into())
            .expect("the coin index is invalid")
    }
}

impl<SwapTask, SEnum, SwapClient> TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
{
    fn on_response<NextState, Label>(
        next: NextState,
        label: Label,
        now: Timestamp,
        querier: QuerierWrapper<'_>,
    ) -> ContinueResult<Self>
    where
        NextState: Enterable + Into<SEnum>,
        Label: Into<String>,
    {
        next.enter(now, querier).and_then(|batch| {
            let emitter = Emitter::of_type(label);
            response::res_continue::<_, _, Self>(
                MessageResponse::messages_with_events(batch, emitter),
                next,
            )
        })
    }
}

impl<SwapTask, SEnum, SwapClient> Enterable for TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
{
    fn enter(&self, now: Timestamp, _querier: QuerierWrapper<'_>) -> Result<Batch> {
        self.enter_state(now)
    }
}

impl<SwapTask, SEnum, SwapClient> Handler for TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
    SwapClient: ExactAmountIn,
    Self: Into<SEnum>,
    SwapExactIn<SwapTask, SEnum, SwapClient>: Into<SEnum>,
{
    type Response = SEnum;
    type SwapResult = SwapTask::Result;

    fn on_response(
        self,
        _resp: Binary,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
        let label = self.spec.label();
        let now = env.block.time;
        if self.last_coin() {
            Self::on_response(SwapExactIn::new(self.spec), label, now, querier)
        } else {
            Self::on_response(self.next(), label, now, querier)
        }
        .into()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    // occasionslly, we get errors from handling the transfer receive message at the remote network
    // we cannot do anything else except keep trying to transfer again
    fn on_error(self, querier: QuerierWrapper<'_>, env: Env) -> HandlerResult<Self> {
        self.on_timeout(querier, env).into()
    }
}

impl<SwapTask, SEnum, SwapClient> Contract for TransferOut<SwapTask, SEnum, SwapClient>
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
        self.spec
            .state(Stage::TransferOut, now, due_projection, querier)
    }
}

impl<SwapTask, SEnum, SwapClient> Display for TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.write_fmt(format_args!("TransferOut at {}", self.spec.label().into()))
    }
}

impl<SwapTask, SEnum, SwapClient> TimeAlarm for TransferOut<SwapTask, SEnum, SwapClient>
where
    SwapTask: SwapTaskT,
{
    fn setup_alarm(&self, r#for: Timestamp) -> Result<Batch> {
        self.spec
            .time_alarm()
            .setup_alarm(r#for)
            .map_err(Into::into)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, SwapTaskNew, SEnum, SEnumNew, SwapClient>
    MigrateSpec<SwapTask, SwapTaskNew, SEnumNew> for TransferOut<SwapTask, SEnum, SwapClient>
where
    Self: Sized,
    SwapTaskNew: SwapTaskT,
    TransferOut<SwapTaskNew, SEnumNew, SwapClient>: Into<SEnumNew>,
{
    type Out = TransferOut<SwapTaskNew, SEnumNew, SwapClient>;

    fn migrate_spec<MigrateFn>(self, migrate_fn: MigrateFn) -> Self::Out
    where
        MigrateFn: FnOnce(SwapTask) -> SwapTaskNew,
    {
        Self::Out::new_with_index(migrate_fn(self.spec), self.coin_index, self.last_coin_index)
    }
}

#[cfg(feature = "migration")]
impl<SwapTask, R, SEnum, SwapClient> InspectSpec<SwapTask, R>
    for TransferOut<SwapTask, SEnum, SwapClient>
{
    fn inspect_spec<InspectFn>(&self, inspect_fn: InspectFn) -> R
    where
        InspectFn: FnOnce(&SwapTask) -> R,
    {
        inspect_fn(&self.spec)
    }
}
