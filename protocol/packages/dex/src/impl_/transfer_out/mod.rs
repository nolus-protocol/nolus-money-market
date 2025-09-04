use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    marker::PhantomData,
};

use serde::{Deserialize, Serialize};

use finance::duration::Duration;
use platform::{
    batch::{Batch, Emitter},
    ica::ErrorResponse as ICAErrorResponse,
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
/// In does it in a single transaction with multiple messages expecting
/// an acknowledgment per message.
#[derive(Serialize, Deserialize)]
#[cfg_attr(test, derive(Clone, Debug, PartialEq, Eq))]
#[serde(
    bound(
        serialize = "SwapTask: Serialize",
        deserialize = "SwapTask: Deserialize<'de> + SwapTaskT"
    ),
    deny_unknown_fields,
    rename_all = "snake_case"
)]
pub struct TransferOut<SwapTask, SEnum, SwapClient> {
    spec: SwapTask,
    acks_left: CoinsNb,
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
        let acks_left = Self::coins_len(&spec);
        Self::internal_new(spec, acks_left)
    }

    fn internal_new(spec: SwapTask, acks_left: CoinsNb) -> Self {
        let ret = Self {
            spec,
            acks_left,
            _state_enum: PhantomData,
            _swap_client: PhantomData,
        };
        debug_assert!(ret.invariant());
        ret
    }

    fn invariant(&self) -> bool {
        let coins_nb = Self::coins_len(&self.spec);
        0 < self.acks_left && self.acks_left <= coins_nb
    }

    fn coins_len(spec: &SwapTask) -> CoinsNb {
        let ret = spec.coins().into_iter().count();
        assert!(ret > 0, "The swap task did not provide any coins!");
        ret.try_into()
            .expect("Functionality doesn't support this many coins!")
    }

    fn generate_requests(&self, now: Timestamp) -> Batch {
        debug_assert_eq!(
            Self::coins_len(&self.spec),
            self.acks_left,
            "calling 'enter_state' past initialization"
        );
        let mut trx = TransferOutTrx::new(self.spec.dex_account(), now);

        self.spec
            .coins()
            .into_iter()
            .for_each(|coin| trx.send(&coin));
        trx.into()
    }

    fn next(self) -> Self {
        debug_assert!(!self.last_ack());

        let acks_left = self.acks_left.checked_sub(1).expect(
            "the method contract precondition `!self.last_ack()` should have been respected",
        );

        Self::internal_new(self.spec, acks_left)
    }

    fn last_ack(&self) -> bool {
        debug_assert!(self.acks_left > 0);
        self.acks_left == 1
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
        msgs: Batch,
    ) -> ContinueResult<Self>
    where
        NextState: Enterable + Into<SEnum>,
        Label: Into<String>,
    {
        let emitter = Emitter::of_type(label);
        response::res_continue::<_, _, Self>(
            MessageResponse::messages_with_event(msgs, emitter),
            next,
        )
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
        Ok(self.generate_requests(now))
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
        if self.last_ack() {
            let next = SwapExactIn::new(self.spec);
            next.enter(env.block.time, querier)
                .and_then(|msgs| Self::on_response(next, label, msgs))
        } else {
            Self::on_response(self.next(), label, Batch::default())
        }
        .into()
    }

    fn on_timeout(self, querier: QuerierWrapper<'_>, env: Env) -> ContinueResult<Self> {
        let state_label = self.spec.label();
        timeout::on_timeout_retry(self, state_label, querier, env)
    }

    // occasionslly, we get errors from handling the transfer receive message at the remote network
    // we cannot do anything else except keep trying to transfer again
    fn on_error(
        self,
        _response: ICAErrorResponse,
        querier: QuerierWrapper<'_>,
        env: Env,
    ) -> HandlerResult<Self> {
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
        Self::Out::new(migrate_fn(self.spec))
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
